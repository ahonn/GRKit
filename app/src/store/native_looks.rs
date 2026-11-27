//! The film looks of other GR cameras, made on the phone from a firmware
//! file that the person downloaded from Ricoh: from GR IV firmware the
//! Cinema and Monochrome looks (`grkit_core::gr4_looks`), from GR II
//! firmware GR II Positive (`grkit_core::gr2_looks`). The camera gets them as
//! Looks, which it lists after it starts again.

use grkit_core::cover::Cover;
use nsh_core::picker::FilePickRequest;
use nsh_core::prelude::*;
use serde_json::json;

use super::looks::off_thread;
use super::{AppState, LookAction};
use crate::domain::{CameraModel, look_media, looks::LookChange};
use crate::strings::tr;

/// The largest file the picker takes: an official GR IV zip has about 35 MB.
const MAX_FIRMWARE_BYTES: u64 = 64 * 1024 * 1024;

/// The camera whose firmware file holds the looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Firmware {
    Gr4,
    Gr2,
}

impl Firmware {
    pub const ALL: [Firmware; 2] = [Firmware::Gr4, Firmware::Gr2];

    /// Ricoh's support page of the camera, which gives the firmware with its
    /// license.
    pub fn page(self) -> &'static str {
        match self {
            Firmware::Gr4 => "https://www.ricoh-imaging.co.jp/english/support/digital/gr4_s.html",
            Firmware::Gr2 => "https://www.ricoh-imaging.co.jp/english/support/digital/gr2_s.html",
        }
    }

    /// The camera's name, as the page names the firmware.
    pub fn camera(self) -> &'static str {
        match self {
            Firmware::Gr4 => "GR IV",
            Firmware::Gr2 => "GR II",
        }
    }

    fn key(self, gr4: &'static str, gr2: &'static str) -> &'static str {
        match self {
            Firmware::Gr4 => gr4,
            Firmware::Gr2 => gr2,
        }
    }

    pub fn title_key(self) -> &'static str {
        self.key("looks.unlock_title", "looks.unlock_title_gr2")
    }

    pub fn row_key(self) -> &'static str {
        self.key("looks.firmware_gr4", "looks.firmware_gr2")
    }

    fn wrong_key(self) -> &'static str {
        self.key("looks.firmware_wrong", "looks.firmware_wrong_gr2")
    }
}

/// A look that a firmware file gives, with the cover it gets on the camera.
#[derive(Clone, Debug, PartialEq)]
pub struct OfferedLook {
    pub id: &'static str,
    pub name: &'static str,
    pub monochrome: bool,
    pub cover: Cover,
}

/// A look made from the firmware file, ready for the camera.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeLook {
    pub id: String,
    pub name: String,
    pub monochrome: bool,
    pub cover: Cover,
    pub grf: Vec<u8>,
}

/// The looks of a read firmware file.
#[derive(Clone, Debug, PartialEq)]
pub struct FoundLooks {
    pub firmware: Firmware,
    /// The version of the firmware.
    pub version: String,
    pub looks: Vec<NativeLook>,
}

#[derive(Clone, Copy)]
pub struct NativeLooksStore {
    found: Signal<Option<FoundLooks>>,
    /// The IDs of the found looks that go to the camera.
    chosen: Signal<Vec<String>>,
    reading: Signal<bool>,
    sending: Signal<bool>,
}

impl Default for NativeLooksStore {
    fn default() -> Self {
        Self {
            found: signal(None),
            chosen: signal(Vec::new()),
            reading: signal(false),
            sending: signal(false),
        }
    }
}

/// The RICOH film box of a look's cover, as `template`, `name` and `number`
/// of a cover JSON. A box cover of these looks has no band.
fn film_box(template: &str, name: &str, number: &str) -> Cover {
    let mut icon = json!({"template": template, "brand": "RICOH", "name": name, "number": number});
    if template == "box" {
        icon["band"] = json!(false);
    }
    Cover::from_json(&icon)
        .expect("the covers of the native looks are valid")
        .film_box()
}

/// The looks that `firmware` gives `model`. GR IV has the Cinema looks built
/// in, and a converted copy would show wrong colors on it; GR II Positive has
/// an effect for the Standard of GR III and GR IIIx and one for GR IV's.
pub fn offered_looks(firmware: Firmware, model: CameraModel) -> Vec<OfferedLook> {
    use grkit_core::{gr2_looks, gr4_looks};
    match firmware {
        Firmware::Gr4 => gr4_looks::LOOKS
            .iter()
            .filter_map(|look| {
                let monochrome = !matches!(look.kind, gr4_looks::Kind::Color);
                (monochrome || model != CameraModel::Gr4).then(|| {
                    let (template, name, number) = look.cover;
                    OfferedLook {
                        id: look.effect_id,
                        name: look.name,
                        monochrome,
                        cover: film_box(template, name, number),
                    }
                })
            })
            .collect(),
        Firmware::Gr2 => vec![OfferedLook {
            id: if model == CameraModel::Gr4 {
                gr2_looks::GR4_EFFECT_ID
            } else {
                gr2_looks::EFFECT_ID
            },
            name: gr2_looks::NAME,
            monochrome: false,
            cover: film_box("cine", "GR II", "POSITIVE"),
        }],
    }
}

/// Read the looks of `offered` from a firmware zip or firmware file of
/// `firmware`.
fn read(firmware: Firmware, offered: &[OfferedLook], bytes: &[u8]) -> Result<FoundLooks, String> {
    use grkit_core::{gr2_looks, gr4_looks};
    let is_gr2 = gr2_looks::detect(bytes);
    let (version, definitions) = match firmware {
        Firmware::Gr4 if !is_gr2 => {
            let image = gr4_looks::Image::read(bytes).map_err(|e| e.to_string())?;
            let definitions = image.definitions().map_err(|e| e.to_string())?;
            (image.source.version.to_owned(), definitions)
        }
        Firmware::Gr2 if is_gr2 => {
            let image = gr2_looks::Image::read(bytes).map_err(|e| e.to_string())?;
            let definitions = image.definitions().map_err(|e| e.to_string())?;
            (image.version().to_owned(), definitions)
        }
        _ => return Err(format!("not a {} firmware file", firmware.camera())),
    };
    let mut looks = Vec::new();
    for definition in definitions {
        let grf =
            grkit_core::effects::compile_definition(&definition).map_err(|e| e.to_string())?;
        let (effect, _) = grkit_core::effects::decode(&grf).map_err(|e| e.to_string())?;
        let Some(look) = offered.iter().find(|look| look.id == effect.id) else {
            continue;
        };
        let grf = look_media::with_cover(&grf, &look.cover).map_err(|e| e.to_string())?;
        looks.push(NativeLook {
            id: effect.id,
            name: look.name.to_owned(),
            monochrome: look.monochrome,
            cover: look.cover.clone(),
            grf,
        });
    }
    Ok(FoundLooks {
        firmware,
        version,
        looks,
    })
}

impl AppState {
    /// The looks of `firmware` that suit the connected camera and that it
    /// lacks.
    pub fn unlock_offered(self, firmware: Firmware) -> Vec<OfferedLook> {
        let Some(model) = self.status().map(|status| status.camera.model) else {
            return Vec::new();
        };
        let Some(snapshot) = self.looks_snapshot() else {
            return Vec::new();
        };
        offered_looks(firmware, model)
            .into_iter()
            .filter(|look| snapshot.get(look.id).is_none())
            .collect()
    }

    pub fn native_looks_found(self) -> Option<FoundLooks> {
        self.native_looks.found.get()
    }

    pub fn native_looks_reading(self) -> bool {
        self.native_looks.reading.get()
    }

    pub fn native_looks_sending(self) -> bool {
        self.native_looks.sending.get()
    }

    /// Whether the found look `id` goes to the camera.
    pub fn native_look_chosen(self, id: &str) -> bool {
        self.native_looks
            .chosen
            .get()
            .iter()
            .any(|chosen| chosen == id)
    }

    /// The number of found looks that go to the camera.
    pub fn native_looks_chosen_count(self) -> usize {
        self.native_looks.chosen.get().len()
    }

    /// Put the found look `id` in or out of what goes to the camera.
    pub fn toggle_native_look(self, id: &str) {
        if self.native_looks.sending.get() {
            return;
        }
        self.native_looks.chosen.update(|chosen| {
            if let Some(at) = chosen.iter().position(|c| c == id) {
                chosen.remove(at);
            } else {
                chosen.push(id.to_owned());
            }
        });
    }

    /// Pick a firmware file of `firmware` and read its looks on a worker
    /// thread. Every look found is chosen.
    pub fn choose_firmware(self, firmware: Firmware) {
        if self.native_looks.reading.get() {
            return;
        }
        let offered = self.unlock_offered(firmware);
        self.root.run(|| {
            spawn(async move {
                let request = FilePickRequest::new(false, 1).max_size(MAX_FIRMWARE_BYTES);
                let Some(file) = pick_files(request).await.into_iter().next() else {
                    return;
                };
                self.native_looks.reading.set(true);
                let bytes = file.data.bytes().to_vec();
                let read = off_thread(move || read(firmware, &offered, &bytes)).await;
                self.native_looks.reading.set(false);
                match read {
                    Some(Ok(found)) if !found.looks.is_empty() => {
                        let ids = found.looks.iter().map(|look| look.id.clone()).collect();
                        self.native_looks.chosen.set(ids);
                        self.native_looks.found.set(Some(found));
                    }
                    other => {
                        if let Some(Err(error)) = other {
                            log::warn!("Could not read the firmware file: {error}");
                        }
                        self.show_toast(tr(firmware.wrong_key(), &[]));
                    }
                }
            });
        });
    }

    /// Send the chosen looks of the read file to the camera, one after
    /// another.
    pub fn send_native_looks(self) {
        let Some(found) = self.native_looks.found.get() else {
            return;
        };
        if self.native_looks.sending.get() {
            return;
        }
        let chosen = self.native_looks.chosen.get();
        let looks: Vec<NativeLook> = found
            .looks
            .into_iter()
            .filter(|look| chosen.contains(&look.id))
            .collect();
        if looks.is_empty() {
            return;
        }
        self.native_looks.sending.set(true);
        self.send_native_look(looks, 0, Vec::new());
    }

    /// Send `looks` from `index` on. A look the camera refuses is skipped,
    /// and its name kept in `failed` for the toast at the end; a camera that
    /// went away stops the rest.
    fn send_native_look(self, looks: Vec<NativeLook>, index: usize, mut failed: Vec<String>) {
        let Some(look) = looks.get(index).cloned() else {
            self.native_looks.sending.set(false);
            self.discard_native_looks();
            self.pop_to(super::Tab::Looks, 0);
            if !failed.is_empty() {
                let names = failed.join(&tr("looks.use_separator", &[]));
                self.show_toast(tr("looks.unlock_failed", &[("names", &names)]));
            }
            return;
        };
        let name = look.name.clone();
        self.change(
            look.id.clone(),
            LookAction::Add,
            LookChange::Content {
                id: look.id,
                grf: look.grf,
                cover: Some(look.cover),
            },
            move |app, ok| {
                if !ok {
                    failed.push(name);
                    if !app.camera_online() {
                        app.native_looks.sending.set(false);
                        return;
                    }
                }
                app.send_native_look(looks, index + 1, failed);
            },
        );
    }

    /// Leave the unlock without sending.
    pub fn discard_native_looks(self) {
        self.native_looks.found.set(None);
        self.native_looks.chosen.set(Vec::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_camera_gets_the_looks_that_suit_it_and_a_wrong_file_reads_as_an_error() {
        let ids = |firmware, model| {
            offered_looks(firmware, model)
                .into_iter()
                .map(|look| {
                    assert!(look.cover.fits());
                    look.id
                })
                .collect::<Vec<_>>()
        };
        let gr3 = ids(Firmware::Gr4, CameraModel::Gr3);
        assert_eq!(gr3.len(), 10);
        assert!(gr3.contains(&"gr4-cinema-yellow"));
        let gr4 = ids(Firmware::Gr4, CameraModel::Gr4);
        assert_eq!(gr4.len(), 8);
        assert!(gr4.iter().all(|id| id.starts_with("gr4-mono-")));
        assert_eq!(ids(Firmware::Gr2, CameraModel::Gr3x), ["gr2-positive-film"]);
        assert_eq!(
            ids(Firmware::Gr2, CameraModel::Gr4),
            ["gr2-positive-film-gr4"]
        );
        let offered = offered_looks(Firmware::Gr4, CameraModel::Gr3);
        assert!(read(Firmware::Gr4, &offered, b"not a firmware file").is_err());
        assert!(read(Firmware::Gr2, &offered, b"not a firmware file").is_err());
    }

    /// The official zips, when the paths are given:
    /// `GRKIT_TEST_FIRMWARE_GR4=… GRKIT_TEST_FIRMWARE_GR2=… cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn the_official_zips_give_every_offered_look() {
        for (firmware, variable, model, count) in [
            (
                Firmware::Gr4,
                "GRKIT_TEST_FIRMWARE_GR4",
                CameraModel::Gr3,
                10,
            ),
            (
                Firmware::Gr4,
                "GRKIT_TEST_FIRMWARE_GR4",
                CameraModel::Gr4,
                8,
            ),
            (
                Firmware::Gr2,
                "GRKIT_TEST_FIRMWARE_GR2",
                CameraModel::Gr3,
                1,
            ),
            (
                Firmware::Gr2,
                "GRKIT_TEST_FIRMWARE_GR2",
                CameraModel::Gr4,
                1,
            ),
        ] {
            let bytes = std::fs::read(std::env::var(variable).unwrap()).unwrap();
            let offered = offered_looks(firmware, model);
            let found = read(firmware, &offered, &bytes).unwrap();
            assert_eq!(found.looks.len(), count, "{firmware:?} on {model:?}");
            let other = match firmware {
                Firmware::Gr4 => Firmware::Gr2,
                Firmware::Gr2 => Firmware::Gr4,
            };
            assert!(read(other, &offered_looks(other, model), &bytes).is_err());
        }
    }
}
