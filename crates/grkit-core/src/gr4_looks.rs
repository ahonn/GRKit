//! GR IV looks from a GR IV firmware file that the user gives: Cinema (Yellow) and Cinema
//! (Green), and the monochrome looks of GR IV Monochrome. The CLI reads the file from a
//! path, and the site reads it in the browser. GRKit ships none of the data; each effect
//! comes from the user's own file.
//!
//! A source is one official build: the SHA-256 of its firmware file, the copy of its data
//! section, and the addresses of the tables that `native_tone` and `native_look` need. A
//! color look keeps the type's look matrix (relative to Standard), its curve set, its
//! MultiAxial set, the default Fn adjustments that the GR IV parameter page shows, and the
//! two ends of the Toning track that the page draws for it (`track_colors`). A
//! monochrome look keeps its curve set and its defaults; GR III makes the gray from its
//! Standard colors (`MONO_MATRIX`), because GR IV Monochrome has a sensor without color
//! filters and its firmware holds no color response.
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};

use crate::effects::Effect;
use crate::firmware;
use crate::native_look::{NativeLook, BANKS, LEVELS};
use crate::native_tone::{Curve, CurveSet, ToneTables};

/// The base address of the RTOS image.
const RTOS_BASE: u32 = 0x5300_0000;
/// The firmware file in an official GR IV zip.
pub const FIRMWARE_FILE: &str = "fwdc248b.bin";
/// The largest file that `Image::read` takes: an official zip or firmware file has about
/// 40 MB.
pub const MAX_FILE_SIZE: usize = 96 * 1024 * 1024;

/// One official GR IV build.
#[derive(Debug)]
pub struct Source {
    /// The version, as the camera shows it.
    pub version: &'static str,
    /// The SHA-256 of `FIRMWARE_FILE`.
    pub firmware_sha256: &'static str,
    /// The data section in RAM, and where the image holds its copy.
    data_start: u32,
    data_source: u32,
    data_size: u32,
    /// The type tables of the look records and of the MultiAxial sets.
    look_table: u32,
    multiaxial_table: u32,
    /// Nine u32 input gains of the key steps.
    gains: u32,
    /// Pointers to the three families; each family is five curve pointers.
    families: u32,
    /// 97 u16 ease weights.
    ease: u32,
    /// The u16 unit of mid gray.
    unit: u32,
    /// The type table of curve sets: three blocks of `curve_set_entries` pointers.
    curve_sets: u32,
    curve_set_entries: u32,
    /// The icon table: a pointer to a 12-byte descriptor for each icon ID (the width in
    /// the high half of word 0, the height in word 1, and the offset of the pixels in the
    /// icon section, in pixels, in word 2).
    icon_table: u32,
}

/// The builds that Ricoh publishes for GR IV and GR IV HDF. The CLI's targets of the same
/// builds hold the same data section and type tables (`grkit-build` checks them).
pub const SOURCES: [Source; 3] = [
    Source {
        version: "1.11",
        firmware_sha256: "a2f664dfca034059eb0fd6e18ab08684c326b4a034d85c164dad7e1ec9b5655f",
        data_start: 0x5500_0000,
        data_source: 0x5437_b640,
        data_size: 0x57480,
        look_table: 0x5505_3780,
        multiaxial_table: 0x5505_3504,
        gains: 0x53b4_d644,
        families: 0x5503_2890,
        ease: 0x5407_b1f4,
        unit: 0x5407_e710,
        curve_sets: 0x5505_3138,
        curve_set_entries: 34,
        icon_table: 0x5501_3410,
    },
    // 1.20 holds 0x26 entries for each model variant in its type tables.
    Source {
        version: "1.20",
        firmware_sha256: "7563d7e462f0c53b6ab2fbab7cdfbbb57151a41192ee82f4f3a2108f9d77650d",
        data_start: 0x5500_0000,
        data_source: 0x543b_3100,
        data_size: 0x58500,
        look_table: 0x5505_4800,
        multiaxial_table: 0x5505_4554,
        gains: 0x53b7_62a4,
        families: 0x5503_3880,
        ease: 0x540b_1eb4,
        unit: 0x540b_59e4,
        curve_sets: 0x5505_4158,
        curve_set_entries: 38,
        icon_table: 0x5501_4280,
    },
    // 1.21 has the data section and the tables of 1.20, and other code addresses.
    Source {
        version: "1.21",
        firmware_sha256: "bf4d6c35f6da1c16ed5f2943af570eab8fef326f7501a2b3ce5413fa55ec5c99",
        data_start: 0x5500_0000,
        data_source: 0x543b_4900,
        data_size: 0x58500,
        look_table: 0x5505_4800,
        multiaxial_table: 0x5505_4554,
        gains: 0x53b7_786c,
        families: 0x5503_3880,
        ease: 0x540b_36a4,
        unit: 0x540b_71d4,
        curve_sets: 0x5505_4158,
        curve_set_entries: 38,
        icon_table: 0x5501_4280,
    },
];

impl Source {
    /// The data section and the type tables, for the check against the CLI targets.
    pub fn layout(&self) -> (u32, u32, u32, [u32; 2]) {
        (
            self.data_start,
            self.data_source,
            self.data_size,
            [self.look_table, self.multiaxial_table],
        )
    }
}

/// How a look becomes an effect.
pub enum Kind {
    /// A color look: the type's look matrix, curve set, and MultiAxial set.
    Color,
    /// A monochrome look of GR IV Monochrome: the curve set of the type in block 2 of the
    /// curve set table, which that camera reads, or linear curves for `None`.
    Monochrome(Option<u32>),
}

/// One look of the source builds.
pub struct Look {
    /// The Image Control type of GR IV.
    pub image_type: u32,
    pub kind: Kind,
    pub effect_id: &'static str,
    pub name: &'static str,
    /// The cover icon: its template, name line, and third line.
    pub cover: (&'static str, &'static str, &'static str),
    /// The defaults that the GR IV parameter page shows (`effects::FN_KEYS` order). The
    /// firmware writes them as instruction immediates into its defaults record, so they
    /// are fixed values, read from the parameter page of each camera.
    pub defaults: [i8; 9],
    /// False for a look that GRKit holds back: `Image::effects` and the site leave it out.
    pub offered: bool,
    /// The icon of the Toning track that the GR IV parameter page draws for the look, or
    /// `None` for a look without Toning. GR IV 1.21 picks it by the look in the slider
    /// setup of its Toning row.
    pub toning_icon: Option<u32>,
}

/// The looks, in the order of the camera lists. A monochrome look whose curve set the file
/// lacks is left out.
pub const LOOKS: [Look; 7] = [
    Look {
        image_type: 18,
        kind: Kind::Color,
        effect_id: "gr4-cinema-yellow",
        name: "Cinema Yellow",
        cover: ("cine", "CINEMA", "YELLOW"),
        defaults: [0, 0, -1, 3, 0, -2, -3, 0, 0],
        offered: true,
        toning_icon: Some(0x263),
    },
    Look {
        image_type: 19,
        kind: Kind::Color,
        effect_id: "gr4-cinema-green",
        name: "Cinema Green",
        cover: ("cine", "CINEMA", "GREEN"),
        defaults: [-2, 0, 0, 1, 0, -3, -3, 0, 0],
        offered: true,
        toning_icon: Some(0x264),
    },
    // GR IV Monochrome: its tone curves follow its defaults and curve sets exactly
    // (live view of GR IV Monochrome 1.10). Grain stays with the Grain module. Held back
    // for now.
    Look {
        image_type: 25,
        kind: Kind::Monochrome(None),
        effect_id: "gr4-mono-standard",
        name: "Mono Standard",
        cover: ("box", "MONO", "STD"),
        defaults: [0; 9],
        offered: false,
        toning_icon: None,
    },
    Look {
        image_type: 32,
        kind: Kind::Monochrome(Some(32)),
        effect_id: "gr4-mono-solid",
        name: "Mono Solid",
        cover: ("box", "MONO", "SOLID"),
        defaults: [0; 9],
        offered: false,
        toning_icon: None,
    },
    Look {
        image_type: 26,
        kind: Kind::Monochrome(None),
        effect_id: "gr4-mono-soft",
        name: "Mono Soft",
        cover: ("box", "MONO", "SOFT"),
        defaults: [0, 0, 1, -2, 0, 0, -1, 0, 0],
        offered: false,
        toning_icon: None,
    },
    Look {
        image_type: 30,
        kind: Kind::Monochrome(Some(30)),
        effect_id: "gr4-mono-high-contrast",
        name: "Mono Hi-Contrast",
        cover: ("box", "MONO", "HI"),
        defaults: [0, 0, 0, 0, 0, 0, 1, 0, 0],
        offered: false,
        toning_icon: None,
    },
    Look {
        image_type: 31,
        kind: Kind::Monochrome(Some(31)),
        effect_id: "gr4-mono-grainy",
        name: "Mono Grainy",
        cover: ("box", "MONO", "GRAINY"),
        defaults: [0; 9],
        offered: false,
        toning_icon: None,
    },
];

/// The block of the curve set table that GR IV Monochrome reads.
const MONOCHROME_BLOCK: u32 = 2;

/// The gray of a monochrome look on GR III: the BT.601 luma of GR III Standard colors, as
/// equal Q13 rows (each row sums to 1).
pub fn mono_matrix() -> [i16; 9] {
    const LUMA: [f64; 3] = [0.299, 0.587, 0.114];
    let standard = crate::fit::STANDARD_LOOK.map(|v| f64::from(v) / 8192.0);
    let row: [i16; 3] = std::array::from_fn(|j| {
        ((0..3).map(|i| LUMA[i] * standard[i * 3 + j]).sum::<f64>() * 8192.0).round() as i16
    });
    std::array::from_fn(|k| row[k % 3])
}

/// The size of a Toning track icon of GR IV, which is also that of GR III.
const TRACK_SIZE: (usize, usize) = (400, 60);
/// The gradient of a Toning track: rows 10 to 13 from column 20 to column 379, black rows
/// above and below it, and a white mark in columns 198 to 201 with a black column on each
/// side.
const TRACK_ROWS: std::ops::RangeInclusive<usize> = 10..=13;
const TRACK_COLUMNS: std::ops::RangeInclusive<usize> = 20..=379;
const TRACK_MARK: std::ops::RangeInclusive<usize> = 197..=202;
/// The largest difference of a gradient pixel from the line between the ends. GR IV
/// dithers its gradients by less than two levels.
const TRACK_TOLERANCE: f64 = 4.0;

/// The two ends of the gradient of a Toning track (RGBA8 pixels of `TRACK_SIZE`), or
/// `None` for an icon of another shape or a gradient that is not a line between two colors.
pub fn track_colors(pixels: &[u8]) -> Option<[[u8; 3]; 2]> {
    let (width, height) = TRACK_SIZE;
    if pixels.len() != width * height * 4 {
        return None;
    }
    let at = |x: usize, y: usize| -> [u8; 4] {
        let i = (y * width + x) * 4;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    };
    let edges = [TRACK_ROWS.start() - 1, TRACK_ROWS.end() + 1];
    let mark = [*TRACK_MARK.start(), *TRACK_MARK.end()];
    let shaped = TRACK_COLUMNS.clone().all(|x| {
        edges.iter().all(|y| at(x, *y) == [0, 0, 0, 255])
            && TRACK_ROWS.clone().all(|y| match at(x, y) {
                p if mark.contains(&x) => p == [0, 0, 0, 255],
                p if TRACK_MARK.contains(&x) => p == [255; 4],
                p => p[3] == 255,
            })
    });
    if !shaped {
        return None;
    }
    let end = |x: usize| -> [f64; 3] {
        std::array::from_fn(|c| {
            TRACK_ROWS
                .clone()
                .map(|y| f64::from(at(x, y)[c]))
                .sum::<f64>()
                / TRACK_ROWS.clone().count() as f64
        })
    };
    let (left, right) = (end(*TRACK_COLUMNS.start()), end(*TRACK_COLUMNS.end()));
    let span = (TRACK_COLUMNS.end() - TRACK_COLUMNS.start()) as f64;
    let linear = TRACK_COLUMNS
        .clone()
        .filter(|x| !TRACK_MARK.contains(x))
        .all(|x| {
            let t = (x - TRACK_COLUMNS.start()) as f64 / span;
            TRACK_ROWS.clone().all(|y| {
                (0..3).all(|c| {
                    let line = left[c] + (right[c] - left[c]) * t;
                    (f64::from(at(x, y)[c]) - line).abs() <= TRACK_TOLERANCE
                })
            })
        });
    let round = |rgb: [f64; 3]| rgb.map(|v| v.round() as u8);
    linear.then(|| [round(left), round(right)])
}

/// The RTOS image of a GR IV firmware file, and the Toning tracks of `LOOKS` that its icons
/// hold.
pub struct Image {
    pub source: &'static Source,
    body: Vec<u8>,
    /// The icon ID and the ends of each Toning track (`track_colors`) of `LOOKS`. A track
    /// whose icon has another shape is left out, and its look has no Toning colors.
    tracks: Vec<(u32, [[u8; 3]; 2])>,
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The firmware file of an official zip, or the bytes of a firmware file.
fn firmware_file(bytes: &[u8]) -> Result<Vec<u8>> {
    if !bytes.starts_with(b"PK") {
        return Ok(bytes.to_vec());
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("Invalid ZIP file")?;
    let index = (0..archive.len())
        .find(|&i| {
            archive.by_index_raw(i).is_ok_and(|entry| {
                entry.name().is_ok_and(|name| {
                    name.rsplit('/')
                        .next()
                        .is_some_and(|file| file.eq_ignore_ascii_case(FIRMWARE_FILE))
                })
            })
        })
        .with_context(|| format!("The ZIP file has no {FIRMWARE_FILE}"))?;
    let mut entry = archive.by_index(index)?;
    ensure!(
        entry.size() as usize <= MAX_FILE_SIZE,
        "{FIRMWARE_FILE} is too large"
    );
    let mut out = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut out)?;
    Ok(out)
}

impl Image {
    /// Reads an official GR IV zip or its firmware file, and checks it against `SOURCES`.
    pub fn read(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= MAX_FILE_SIZE, "The file is too large");
        let file = firmware_file(bytes)?;
        let identity = sha256(&file);
        let Some(source) = SOURCES.iter().find(|s| s.firmware_sha256 == identity) else {
            bail!(
                "This is not a supported GR IV firmware file. Supported versions: {}",
                SOURCES.map(|s| s.version).join(", ")
            );
        };
        let decoded = firmware::decode(&file)?;
        let body = firmware::rtos_section(&decoded)?.to_vec();
        let mut image = Self {
            source,
            body,
            tracks: Vec::new(),
        };
        let icons = firmware::icon_section(&decoded)?;
        image.tracks = LOOKS
            .iter()
            .filter_map(|look| look.toning_icon)
            .filter_map(|id| Some((id, track_colors(image.icon(icons, id)?)?)))
            .collect();
        Ok(image)
    }

    /// The RGBA8 pixels of icon `id` if it has `TRACK_SIZE`, from the icon section.
    fn icon<'a>(&self, icons: &'a [u8], id: u32) -> Option<&'a [u8]> {
        let descriptor = self
            .word(self.source.icon_table.checked_add(id.checked_mul(4)?)?)
            .ok()?;
        let width = self.word(descriptor).ok()? >> 16;
        let height = self.word(descriptor.checked_add(4)?).ok()?;
        let offset = self.word(descriptor.checked_add(8)?).ok()? as usize;
        if (width as usize, height as usize) != TRACK_SIZE {
            return None;
        }
        let start = offset.checked_mul(4)?;
        icons.get(start..start.checked_add(TRACK_SIZE.0 * TRACK_SIZE.1 * 4)?)
    }

    /// The ends of the Toning track of the look of Image Control type `image_type`, if the
    /// file holds it.
    pub fn toning_colors(&self, image_type: u32) -> Option<[[u8; 3]; 2]> {
        let id = LOOKS
            .iter()
            .find(|look| look.image_type == image_type)?
            .toning_icon?;
        self.tracks
            .iter()
            .find(|(icon, _)| *icon == id)
            .map(|(_, colors)| *colors)
    }

    /// RTOS memory. The data section reads from its copy in the image.
    fn bytes(&self, address: u32, size: usize) -> Result<&[u8]> {
        let s = self.source;
        let address = if (s.data_start..s.data_start + s.data_size).contains(&address) {
            address - s.data_start + s.data_source
        } else {
            address
        };
        let offset = address
            .checked_sub(RTOS_BASE)
            .context("Address outside the firmware")? as usize;
        self.body
            .get(offset..offset + size)
            .context("Address outside the firmware")
    }

    fn word(&self, address: u32) -> Result<u32> {
        Ok(u32::from_le_bytes(self.bytes(address, 4)?.try_into()?))
    }

    fn u16s(&self, address: u32, n: usize) -> Result<Vec<u16>> {
        Ok(self
            .bytes(address, n * 2)?
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect())
    }

    fn curve(&self, address: u32) -> Result<Curve> {
        let values = self.u16s(address, 256)?;
        Ok(std::array::from_fn(|i| values[i]))
    }

    /// The tone tables, checked for their shape.
    pub fn tone_tables(&self) -> Result<ToneTables> {
        let s = self.source;
        let mut gains = [0; 9];
        for (i, gain) in gains.iter_mut().enumerate() {
            *gain = self.word(s.gains + 4 * i as u32)?;
        }
        let mut families = [[[0; 256]; 5]; 3];
        for (f, family) in families.iter_mut().enumerate() {
            let pointers = self.word(s.families + 4 * f as u32)?;
            for (c, slot) in family.iter_mut().enumerate() {
                *slot = self.curve(self.word(pointers + 4 * c as u32)?)?;
            }
        }
        let ease = self.u16s(s.ease, 97)?;
        let tables = ToneTables {
            gains,
            families,
            ease: std::array::from_fn(|i| ease[i]),
            unit: self.u16s(s.unit, 1)?[0],
        };
        tables
            .validate()
            .with_context(|| format!("Unexpected tone tables in GR IV {}", s.version))?;
        Ok(tables)
    }

    /// The curve set of an Image Control type. The three blocks of the type table agree.
    pub fn curve_set(&self, image_type: u32) -> Result<CurveSet> {
        let address = self.curve_set_address(image_type, 0)?;
        for block in 1..3 {
            ensure!(
                self.curve_set_address(image_type, block)? == address,
                "The curve set blocks differ for type {image_type}"
            );
        }
        self.read_curve_set(image_type, address)
    }

    /// The curve set of an Image Control type in one block of the type table.
    pub fn curve_set_in(&self, image_type: u32, block: u32) -> Result<CurveSet> {
        self.read_curve_set(image_type, self.curve_set_address(image_type, block)?)
    }

    fn curve_set_address(&self, image_type: u32, block: u32) -> Result<u32> {
        let s = self.source;
        ensure!(
            image_type < s.curve_set_entries && block < 3,
            "Image Control type {image_type} out of range"
        );
        self.word(s.curve_sets + 4 * (block * s.curve_set_entries + image_type))
    }

    fn read_curve_set(&self, image_type: u32, address: u32) -> Result<CurveSet> {
        let flags = self.bytes(address + 4, 2)?;
        let set = CurveSet {
            enabled: flags[0] != 0,
            weight: flags[1],
            curves: [
                self.curve(self.word(address + 8)?)?,
                self.curve(self.word(address + 12)?)?,
                self.curve(self.word(address + 16)?)?,
            ],
        };
        ensure!(
            set.weight <= 3
                && set.curves.iter().all(|c| {
                    c.windows(2).all(|pair| pair[0] <= pair[1] + 64) && c[255] <= 0x4000
                }),
            "Unexpected curve set for type {image_type}"
        );
        Ok(set)
    }

    /// The look matrix (Q13, at record +4) of an Image Control type.
    fn look_matrix(&self, image_type: u32) -> Result<[i16; 9]> {
        let record = self.word(self.source.look_table + 4 * image_type)?;
        let bytes = self.bytes(self.word(record + 4)?, 18)?;
        Ok(std::array::from_fn(|k| {
            i16::from_le_bytes([bytes[2 * k], bytes[2 * k + 1]])
        }))
    }

    /// The MultiAxial set of an Image Control type.
    fn multiaxial(&self, image_type: u32) -> Result<NativeLook> {
        let set = self.word(self.source.multiaxial_table + 4 * image_type)?;
        let pointer = |k: u32| self.word(set + 4 * k);
        let bounds = self.u16s(pointer(0)?, 16)?;
        let selectors = self.bytes(pointer(1)?, 20)?;
        let control = self.u16s(pointer(4)?, 21)?;
        let matrices = self.bytes(self.word(pointer(2)? + 0x10)?, BANKS * LEVELS * 36)?;
        let at = |bank: usize, level: usize, k: usize| {
            let offset = ((bank * LEVELS + level) * 9 + k) * 4;
            i32::from_le_bytes(matrices[offset..offset + 4].try_into().unwrap())
        };
        let levels: [[[i32; 9]; LEVELS]; BANKS] =
            std::array::from_fn(|b| std::array::from_fn(|l| std::array::from_fn(|k| at(b, l, k))));
        NativeLook::from_levels(
            std::array::from_fn(|k| bounds[k]),
            std::array::from_fn(|k| selectors[k]),
            std::array::from_fn(|k| control[k]),
            &levels,
        )
        .with_context(|| format!("Unexpected MultiAxial set for type {image_type}"))
    }

    /// An effect that gives Image Control type `image_type` to a camera whose Standard
    /// look matrix is `standard` (`fit::STANDARD_LOOK` for GR III).
    ///
    /// The matrix is the type's look matrix relative to the image's Standard (type 11), on
    /// the target's Standard. The curves are the type's curve set after its weight; the
    /// target camera applies them to its own base curve, which its tone generator makes
    /// from the Fn adjustments. A type without its own curve set keeps linear curves.
    pub fn effect(
        &self,
        image_type: u32,
        defaults: [i8; 9],
        standard: &[i16; 9],
        id: &str,
        name: &str,
    ) -> Result<Effect> {
        let set = self.curve_set(image_type)?;
        let curves = if set.enabled {
            set.mixed()
        } else {
            [std::array::from_fn(|i| (i * 16383 / 255) as u16); 3]
        };
        ensure!(
            curves.iter().flatten().all(|v| *v <= 16383),
            "A curve of type {image_type} leaves the 14-bit range"
        );
        let source = rows(&self.look_matrix(image_type)?);
        let base = rows(&self.look_matrix(11)?);
        let matrix = product(&product(&source, &inverse(&base)?), &rows(standard));
        let mut q13 = [0i16; 9];
        for (k, slot) in q13.iter_mut().enumerate() {
            let v = (matrix[k / 3][k % 3] * 8192.0).round();
            ensure!(
                (-32768.0..=32767.0).contains(&v),
                "The look matrix leaves the Q13 range"
            );
            *slot = v as i16;
        }
        Ok(Effect {
            id: id.to_owned(),
            name: name.to_owned(),
            flags: 0,
            matrix: q13,
            curves,
            post_matrix: None,
            selective: None,
            hue_sectors: None,
            native_look: Some(self.multiaxial(image_type)?),
            fn_defaults: Some(defaults),
            toning_colors: self.toning_colors(image_type),
        })
    }

    /// The effect of a monochrome look, or `None` when the file lacks its curve set.
    fn monochrome(&self, look: &Look, set: Option<u32>) -> Result<Option<Effect>> {
        let curve = match set {
            Some(image_type) => {
                let set = self.curve_set_in(image_type, MONOCHROME_BLOCK)?;
                if !set.enabled {
                    return Ok(None);
                }
                let [red, green, blue] = set.mixed();
                ensure!(
                    red == green && green == blue,
                    "The curve set of type {image_type} is not gray"
                );
                // A set may end at 16384; an effect curve holds 14 bits.
                green.map(|v| v.min(16383))
            }
            None => std::array::from_fn(|i| (i * 16383 / 255) as u16),
        };
        Ok(Some(Effect {
            id: look.effect_id.to_owned(),
            name: look.name.to_owned(),
            flags: crate::effects::MONOCHROME,
            matrix: mono_matrix(),
            curves: [curve; 3],
            post_matrix: None,
            selective: None,
            hue_sectors: None,
            native_look: None,
            fn_defaults: look
                .defaults
                .iter()
                .any(|v| *v != 0)
                .then_some(look.defaults),
            toning_colors: None,
        }))
    }

    /// The effects of the offered `LOOKS` for GR III that the file holds.
    pub fn effects(&self) -> Result<Vec<(&'static Look, Effect)>> {
        let mut effects = self.all_effects()?;
        effects.retain(|(look, _)| look.offered);
        Ok(effects)
    }

    /// The effects of every look of `LOOKS` for GR III that the file holds, the held-back
    /// looks included.
    pub fn all_effects(&self) -> Result<Vec<(&'static Look, Effect)>> {
        let mut out = Vec::new();
        for look in &LOOKS {
            let effect = match look.kind {
                Kind::Color => Some(self.effect(
                    look.image_type,
                    look.defaults,
                    &crate::fit::STANDARD_LOOK,
                    look.effect_id,
                    look.name,
                )?),
                Kind::Monochrome(set) => self.monochrome(look, set)?,
            };
            if let Some(effect) = effect {
                out.push((look, effect));
            }
        }
        Ok(out)
    }

    /// The effect.json definition of each offered look that the file holds, for GR III.
    pub fn definitions(&self) -> Result<Vec<Value>> {
        self.effects()?
            .into_iter()
            .map(|(look, effect)| {
                let mut value = effect.to_json();
                if let Some(object) = value.as_object_mut() {
                    object.insert("schema".to_owned(), json!("grkit-effect-v1"));
                    let (template, name, number) = look.cover;
                    let mut icon = json!({"template": template, "brand": "RICOH", "name": name, "number": number});
                    if template == "box" {
                        icon["band"] = json!(false);
                    }
                    object.insert("icon".to_owned(), icon);
                    object.insert(
                        "attribution".to_owned(),
                        json!({
                            "origin": "native-look",
                            "source": format!("GR IV {} firmware of the user", self.source.version),
                        }),
                    );
                }
                Ok(value)
            })
            .collect()
    }
}

fn rows(m: &[i16; 9]) -> [[f64; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| f64::from(m[r * 3 + c]) / 8192.0))
}

fn product(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| a[r][k] * b[k][c]).sum()))
}

fn inverse(m: &[[f64; 3]; 3]) -> Result<[[f64; 3]; 3]> {
    let cofactor = |r: usize, c: usize| {
        let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
        let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
        m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
    };
    let det: f64 = (0..3).map(|c| m[0][c] * cofactor(0, c)).sum();
    ensure!(det.abs() > 1e-9, "Singular look matrix");
    Ok(std::array::from_fn(|r| {
        std::array::from_fn(|c| cofactor(c, r) / det)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn other_files_name_the_supported_versions() {
        let error = Image::read(b"not firmware").err().unwrap().to_string();
        assert!(error.contains("1.11, 1.20"), "{error}");
        // A zip without the firmware file.
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("readme.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut zip, b"x").unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let error = Image::read(&bytes).err().unwrap().to_string();
        assert!(error.contains(FIRMWARE_FILE), "{error}");
    }

    /// A track of `TRACK_SIZE` with the gradient of `color` at each column, the borders,
    /// and the mark.
    fn track(color: impl Fn(usize) -> [u8; 3]) -> Vec<u8> {
        let (width, height) = TRACK_SIZE;
        let mut pixels = vec![0; width * height * 4];
        let mut put = |x: usize, y: usize, p: [u8; 4]| {
            let i = (y * width + x) * 4;
            pixels[i..i + 4].copy_from_slice(&p);
        };
        for x in TRACK_COLUMNS {
            put(x, 9, [0, 0, 0, 255]);
            put(x, 14, [0, 0, 0, 255]);
            for y in TRACK_ROWS {
                let [r, g, b] = color(x);
                put(x, y, [r, g, b, 255]);
                if TRACK_MARK.contains(&x) {
                    let edge = x == *TRACK_MARK.start() || x == *TRACK_MARK.end();
                    put(x, y, if edge { [0, 0, 0, 255] } else { [255; 4] });
                }
            }
        }
        pixels
    }

    #[test]
    fn a_track_gives_the_ends_of_its_gradient() {
        let lerp = |a: u8, b: u8, x: usize| {
            let t = (x - 20) as f64 / 359.0;
            (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8
        };
        let green = |x| [lerp(154, 103, x), lerp(167, 165, x), lerp(157, 116, x)];
        assert_eq!(
            track_colors(&track(green)),
            Some([[154, 167, 157], [103, 165, 116]])
        );
        // A dither of two levels keeps the ends.
        let dithered = |x: usize| green(x).map(|v| v.saturating_add((x % 3) as u8));
        assert!(track_colors(&track(dithered)).is_some());
        // A gradient through a third color, an icon without the mark, or of another size
        // gives none.
        let bent = |x: usize| {
            if x < 200 {
                [76, 121, 255]
            } else {
                [153, 102, 0]
            }
        };
        assert_eq!(track_colors(&track(bent)), None);
        let mut plain = track(green);
        let i = (11 * TRACK_SIZE.0 + 199) * 4;
        plain[i..i + 4].copy_from_slice(&[154, 167, 157, 255]);
        assert_eq!(track_colors(&plain), None);
        assert_eq!(track_colors(&plain[4..]), None);
    }
}
