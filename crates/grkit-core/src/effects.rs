//! Data-only Image Control definitions and GRFX0001 encoding.
//!
//! `abi::GRF_FIELDS` gives the layout of the file. The extension is the features, the
//! post-curve matrix, the selective color (see `selective`), and reserved bytes. An
//! effect without features has zeros in the whole extension. With feature 8, the hue
//! sectors (see `hue`) follow the icon; the rest of the file stays zero. Feature 16 adds
//! hue levels to feature 8: the file grows to `SIZE_WITH_LEVELS` bytes, and the bytes
//! from `SIZE` hold the sectors of levels 1 to 3, then zeros. Feature 32, a native look
//! (`native_look`), excludes features 8 and 16: its MultiAxial set fills the hue area and
//! the bytes from `SIZE` of a `SIZE_WITH_LEVELS` file. Feature 64 gives the effect default
//! Fn adjustments: nine i8 at `FN_DEFAULTS` of a `SIZE_WITH_LEVELS` file, after the data of
//! features 16 and 32, so it goes with any effect, monochrome ones included.

use anyhow::{bail, ensure, Context, Result};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fmt;

use crate::abi::{
    GRF_BYTES, GRF_CRC, GRF_CURVES, GRF_FEATURES, GRF_FLAGS, GRF_HUE, GRF_ICON, GRF_ID, GRF_MAGIC,
    GRF_MATRIX, GRF_NAME, GRF_PADDING, GRF_POST, GRF_RESERVED, GRF_SELECTIVE,
};
use crate::hue;
use crate::native_look::{self, NativeLook};
use crate::selective::Selective;

pub const MAGIC: &[u8; 8] = b"GRFX0001";
pub const SIZE: usize = 8192;
/// The size of an effect with hue levels (feature 16), a native look (feature 32), or Fn
/// defaults (feature 64).
pub const SIZE_WITH_LEVELS: usize = 8704;
pub const ICON_SIZE: usize = 6400;
pub const ICON_OFFSET: usize = GRF_ICON;
/// The longest effect ID and the longest effect name, in characters.
pub const ID_MAX: usize = 31;
pub const NAME_MAX: usize = 16;
pub const MONOCHROME: u32 = 1;
pub const SCHEMA: &str = "grkit-effect-v1";
pub const FEATURE_POST: u32 = 1;
pub const FEATURE_SELECTIVE: u32 = 2;
/// Hue sectors for the MultiAxial stage (`hue`), at HUE_OFFSET.
pub const FEATURE_HUE_SECTORS: u32 = 8;
/// Hue levels (`hue::LEVELS`), with feature 8, at `SIZE`.
pub const FEATURE_HUE_LEVELS: u32 = 16;
/// A native look (`native_look`), without features 8 and 16.
pub const FEATURE_NATIVE_LOOK: u32 = 32;
/// Default Fn adjustments, at `FN_DEFAULTS`.
pub const FEATURE_FN_DEFAULTS: u32 = 64;
const FEATURES: u32 = FEATURE_POST
    | FEATURE_SELECTIVE
    | FEATURE_HUE_SECTORS
    | FEATURE_HUE_LEVELS
    | FEATURE_NATIVE_LOOK
    | FEATURE_FN_DEFAULTS;
/// The Fn adjustments in the order of the record (`abi::RECORD_PARAMETERS`).
pub const FN_KEYS: [&str; 9] = [
    "saturation",
    "hue",
    "key",
    "contrast",
    "highlight",
    "shadow",
    "sharpness",
    "shading",
    "clarity",
];
/// The largest shown value of an Fn adjustment.
pub const FN_MAX: i8 = 4;
/// The default Fn adjustments (feature 64): one i8 for each of `FN_KEYS`, -4..=4, the value
/// that the Fn page shows before a change. The camera applies default + change.
pub const FN_DEFAULTS: usize = native_look::END;
pub const FN_DEFAULTS_END: usize = FN_DEFAULTS + FN_KEYS.len();
const _: () = assert!(SIZE + hue::LEVEL_SIZE <= FN_DEFAULTS && FN_DEFAULTS_END <= SIZE_WITH_LEVELS);
/// The hue sectors use the start of the reserved tail of the file.
pub const HUE_OFFSET: usize = GRF_HUE;
/// Optional keys of both a definition and a decoded effect. A definition writes
/// `selective_color` in degrees; a decoded effect holds its GRF integers.
const EXTENSION_KEYS: [&str; 5] = [
    "post_matrix",
    "selective_color",
    "hue_sectors",
    "native_look",
    "fn_defaults",
];

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("JSON data with unique object keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|v| UniqueValue(Value::Number(v)))
                    .ok_or_else(|| E::custom("Expected finite JSON number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut result = Vec::new();
                while let Some(UniqueValue(value)) = sequence.next_element()? {
                    result.push(value);
                }
                Ok(UniqueValue(Value::Array(result)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut result = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if result.contains_key(&key) {
                        return Err(de::Error::custom(format!("Duplicate JSON key: {key}")));
                    }
                    let UniqueValue(value) = map.next_value()?;
                    result.insert(key, value);
                }
                Ok(UniqueValue(Value::Object(result)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

pub fn parse_json(bytes: &[u8]) -> Result<Value> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let UniqueValue(value) = UniqueValue::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

fn has_keys(value: &Value, keys: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|k| object.contains_key(*k))
    })
}

/// Renders the `icon` of an effect definition: a label tile (`label`, `shape`),
/// a film box (`template` "box"), or a motion picture film label (`template` "cine").
/// A cover can leave out `number`, and then shows a large name.
pub fn icon_pixels(icon: &Value) -> Result<Vec<u8>> {
    let text = |key: &str| {
        icon[key]
            .as_str()
            .with_context(|| format!("Expected icon {key}"))
    };
    // A present number has 1 to 12 characters. Without the key, the cover has none.
    let number = || -> Result<&str> {
        match icon.get("number") {
            None => Ok(""),
            Some(_) => {
                let number = text("number")?;
                ensure!(
                    !number.is_empty(),
                    "Expected a cover number, or no number key"
                );
                Ok(number)
            }
        }
    };
    let keys = |required: &[&str]| {
        let mut with_number = required.to_vec();
        with_number.push("number");
        has_keys(icon, required) || has_keys(icon, &with_number)
    };
    match icon.get("template").map(|t| t.as_str().unwrap_or_default()) {
        None => {
            ensure!(
                has_keys(icon, &["label", "shape"]),
                "Expected icon label and shape"
            );
            crate::artwork::render_tile(text("label")?, text("shape")?)
        }
        Some("box") => {
            ensure!(
                keys(&["template", "brand", "name", "band"]),
                "Expected box icon brand, name, band, and an optional number"
            );
            let band = icon["band"].as_bool().context("Expected icon band")?;
            crate::artwork::render_box(text("brand")?, text("name")?, number()?, band)
        }
        Some("cine") => {
            ensure!(
                keys(&["template", "brand", "name"]),
                "Expected cine icon brand, name, and an optional number"
            );
            crate::artwork::render_cine(text("brand")?, text("name")?, number()?)
        }
        Some(_) => bail!("Expected icon template box or cine"),
    }
}

/// `required` keys, any of the `optional` keys, and nothing else.
fn has_keys_with(value: &Value, required: &[&str], optional: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        required.iter().all(|k| object.contains_key(*k))
            && object
                .keys()
                .all(|k| required.contains(&k.as_str()) || optional.contains(&k.as_str()))
    })
}

/// A decoded effect: every field of a GRFX0001 file except the icon.
///
/// JSON appears only at the edges: effect definitions (`compile_definition`) and the
/// decoded JSON form (`from_json`, `to_json`), which holds the GRF integers.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub id: String,
    pub name: String,
    /// 0 or `MONOCHROME`.
    pub flags: u32,
    /// The signed Q13 effect matrix, row-major.
    pub matrix: [i16; 9],
    /// The 14-bit curve of each channel.
    pub curves: [[u16; 256]; 3],
    /// The signed Q13 post-curve matrix (feature 1).
    pub post_matrix: Option<[i16; 9]>,
    /// Feature 2.
    pub selective: Option<Selective>,
    /// Feature 8, and feature 16 with levels.
    pub hue_sectors: Option<hue::HueSectors>,
    /// Feature 32.
    pub native_look: Option<NativeLook>,
    /// Feature 64, in the order of `FN_KEYS`.
    pub fn_defaults: Option<[i8; 9]>,
}

/// The `fn_defaults` object: each of `FN_KEYS` with -4..=4.
fn read_fn_defaults(value: &Value) -> Result<[i8; 9]> {
    let message = "Expected fn_defaults with saturation, hue, key, contrast, highlight, shadow, sharpness, shading, and clarity of -4 to 4";
    let given = value.as_object().context(message)?;
    ensure!(
        given.len() == FN_KEYS.len() && FN_KEYS.iter().all(|k| given.contains_key(*k)),
        "{message}"
    );
    let mut defaults = [0i8; 9];
    for (slot, key) in defaults.iter_mut().zip(FN_KEYS) {
        *slot = given[key]
            .as_i64()
            .filter(|v| (-i64::from(FN_MAX)..=i64::from(FN_MAX)).contains(v))
            .context(message)? as i8;
    }
    Ok(defaults)
}

fn fn_defaults_json(defaults: &[i8; 9]) -> Value {
    Value::Object(
        FN_KEYS
            .iter()
            .zip(defaults)
            .map(|(k, v)| ((*k).to_owned(), json!(v)))
            .collect(),
    )
}

/// The matrix and the curves of `matrix` and `curves`, JSON arrays of GRF integers.
fn coefficients(matrix: &Value, curves: &Value) -> Result<([i16; 9], [[u16; 256]; 3])> {
    let matrix = matrix
        .as_array()
        .context("Expected matrix and curve arrays")?;
    let curves = curves
        .as_array()
        .context("Expected matrix and curve arrays")?;
    ensure!(
        matrix.len() == 9
            && matrix
                .iter()
                .all(|v| v.as_i64().is_some_and(|n| (-32768..=32767).contains(&n))),
        "Expected nine signed Q13 matrix coefficients"
    );
    ensure!(
        curves.len() == 3
            && curves.iter().all(|c| c.as_array().is_some_and(
                |c| c.len() == 256 && c.iter().all(|v| v.as_u64().is_some_and(|n| n <= 16383))
            )),
        "Expected three 256-sample 14-bit curves"
    );
    Ok((
        std::array::from_fn(|i| matrix[i].as_i64().unwrap() as i16),
        std::array::from_fn(|c| std::array::from_fn(|i| curves[c][i].as_u64().unwrap() as u16)),
    ))
}

impl Effect {
    /// Reads the fields that a definition and the decoded JSON form share. `curves` are
    /// the GRF curves, and `selective` the selective color, because the two forms write
    /// them differently.
    fn read(value: &Value, curves: &Value, selective: Option<Selective>) -> Result<Self> {
        let flags = value
            .get("flags")
            .map_or(Some(0), Value::as_u64)
            .context("Unsupported effect flags")?;
        ensure!(flags <= MONOCHROME as u64, "Unsupported effect flags");
        let id = value["effect_id"].as_str().unwrap_or("");
        let name = value["name"]
            .as_str()
            .context("Expected an ASCII effect name")?;
        check_identity(id, name)?;
        let (matrix, curves) = coefficients(&value["matrix"], curves)?;
        let post_matrix = value
            .get("post_matrix")
            .map(|matrix| -> Result<[i16; 9]> {
                let error = "Expected nine signed Q13 post-matrix coefficients";
                let values = matrix.as_array().filter(|m| m.len() == 9).context(error)?;
                let mut post = [0; 9];
                for (slot, value) in post.iter_mut().zip(values) {
                    *slot = value
                        .as_i64()
                        .and_then(|n| i16::try_from(n).ok())
                        .context(error)?;
                }
                Ok(post)
            })
            .transpose()?;
        let effect = Self {
            id: id.to_owned(),
            name: name.to_owned(),
            flags: flags as u32,
            matrix,
            curves,
            post_matrix,
            selective,
            hue_sectors: value
                .get("hue_sectors")
                .map(hue::HueSectors::from_json)
                .transpose()?,
            native_look: value
                .get("native_look")
                .map(NativeLook::from_json)
                .transpose()?,
            fn_defaults: value.get("fn_defaults").map(read_fn_defaults).transpose()?,
        };
        effect.check()?;
        Ok(effect)
    }

    /// Reads the decoded JSON form (`to_json`).
    pub fn from_json(value: &Value) -> Result<Self> {
        let mut optional = vec!["flags"];
        optional.extend(EXTENSION_KEYS);
        ensure!(
            has_keys_with(
                value,
                &["effect_id", "name", "matrix", "curves"],
                &optional
            ),
            "Expected effect_id, name, matrix, curves, and optional flags, post_matrix, selective_color, hue_sectors, native_look, and fn_defaults"
        );
        let selective = value
            .get("selective_color")
            .map(Selective::from_json)
            .transpose()?;
        Self::read(value, &value["curves"], selective)
    }

    /// The decoded JSON form: the GRF integers of each field, and the optional keys of
    /// the features that the effect has.
    pub fn to_json(&self) -> Value {
        let mut value = json!({"effect_id": self.id, "name": self.name, "flags": self.flags,
            "matrix": self.matrix, "curves": self.curves.map(Vec::from)});
        if let Some(post) = self.post_matrix {
            value["post_matrix"] = json!(post);
        }
        if let Some(selective) = &self.selective {
            value["selective_color"] = selective.to_json();
        }
        if let Some(sectors) = &self.hue_sectors {
            value["hue_sectors"] = sectors.to_json();
        }
        if let Some(look) = &self.native_look {
            value["native_look"] = look.to_json();
        }
        if let Some(defaults) = &self.fn_defaults {
            value["fn_defaults"] = fn_defaults_json(defaults);
        }
        value
    }

    pub fn monochrome(&self) -> bool {
        self.flags == MONOCHROME
    }

    fn features(&self) -> u32 {
        let mut features = 0;
        if self.post_matrix.is_some() {
            features |= FEATURE_POST;
        }
        if self.selective.is_some() {
            features |= FEATURE_SELECTIVE;
        }
        if let Some(sectors) = &self.hue_sectors {
            features |= FEATURE_HUE_SECTORS;
            if sectors.levels.is_some() {
                features |= FEATURE_HUE_LEVELS;
            }
        }
        if self.native_look.is_some() {
            features |= FEATURE_NATIVE_LOOK;
        }
        if self.fn_defaults.is_some() {
            features |= FEATURE_FN_DEFAULTS;
        }
        features
    }

    /// Checks the fields that the parts do not check themselves.
    fn check(&self) -> Result<()> {
        ensure!(self.flags <= MONOCHROME, "Unsupported effect flags");
        ensure!(
            self.hue_sectors.is_none() || self.native_look.is_none(),
            "An effect has hue sectors or a native look, not both"
        );
        if let Some(look) = &self.native_look {
            look.check()?;
        }
        if let Some(defaults) = &self.fn_defaults {
            ensure!(
                defaults.iter().all(|v| (-FN_MAX..=FN_MAX).contains(v)),
                "Fn defaults must be -4..=4"
            );
        }
        check_identity(&self.id, &self.name)?;
        ensure!(
            self.curves.iter().flatten().all(|v| *v <= 16383),
            "Expected three 256-sample 14-bit curves"
        );
        if self.monochrome() {
            ensure!(
                self.post_matrix.is_none(),
                "Monochrome effects cannot have a post matrix"
            );
            ensure!(
                self.selective.is_none(),
                "Monochrome effects cannot have selective color"
            );
            ensure!(
                self.hue_sectors.is_none() && self.native_look.is_none(),
                "Monochrome effects cannot have hue sectors or a native look"
            );
            let m = &self.matrix;
            let c = &self.curves;
            ensure!(
                m[..3] == m[3..6] && m[..3] == m[6..9] && c[0] == c[1] && c[0] == c[2],
                "Monochrome requires equal matrix rows and RGB curves"
            );
        }
        Ok(())
    }

    /// The GRFX0001 file of the effect with `icon`, 40 x 40 RGBA8 pixels.
    pub fn encode(&self, icon: &[u8]) -> Result<Vec<u8>> {
        self.check()?;
        check_icon(icon)?;
        let features = self.features();
        let size =
            if features & (FEATURE_HUE_LEVELS | FEATURE_NATIVE_LOOK | FEATURE_FN_DEFAULTS) != 0 {
                SIZE_WITH_LEVELS
            } else {
                SIZE
            };
        let mut blob = vec![0; size];
        blob[GRF_MAGIC..GRF_MAGIC + 8].copy_from_slice(MAGIC);
        blob[GRF_FEATURES..GRF_FEATURES + 4].copy_from_slice(&features.to_le_bytes());
        for (index, value) in self.post_matrix.unwrap_or([0; 9]).iter().enumerate() {
            let at = GRF_POST + index * 2;
            blob[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
        if let Some(selective) = &self.selective {
            blob[GRF_SELECTIVE..GRF_RESERVED].copy_from_slice(&selective.to_bytes());
        }
        if let Some(sectors) = &self.hue_sectors {
            blob[HUE_OFFSET..HUE_OFFSET + hue::SIZE].copy_from_slice(&sectors.to_bytes());
            if let Some(levels) = sectors.level_bytes() {
                blob[SIZE..SIZE + hue::LEVEL_SIZE].copy_from_slice(&levels);
            }
        }
        if let Some(look) = &self.native_look {
            look.write(&mut blob);
        }
        if let Some(defaults) = &self.fn_defaults {
            for (slot, value) in blob[FN_DEFAULTS..FN_DEFAULTS_END].iter_mut().zip(defaults) {
                *slot = *value as u8;
            }
        }
        blob[GRF_BYTES..GRF_BYTES + 4].copy_from_slice(&(size as u32).to_le_bytes());
        blob[GRF_FLAGS..GRF_FLAGS + 4].copy_from_slice(&self.flags.to_le_bytes());
        for (offset, text) in [(GRF_ID, &self.id), (GRF_NAME, &self.name)] {
            blob[offset..offset + text.len()].copy_from_slice(text.as_bytes());
        }
        // The matrix, two bytes of padding, then the curves.
        for (index, value) in self.matrix.iter().enumerate() {
            let at = GRF_MATRIX + index * 2;
            blob[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
        for (index, value) in self.curves.iter().flatten().enumerate() {
            let at = GRF_CURVES + index * 2;
            blob[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
        blob[GRF_ICON..GRF_ICON + ICON_SIZE].copy_from_slice(icon);
        let crc = checksum(&blob);
        blob[GRF_CRC..GRF_CRC + 4].copy_from_slice(&crc.to_le_bytes());
        Ok(blob)
    }
}

pub fn valid_id(id: &str) -> bool {
    (1..=ID_MAX).contains(&id.len())
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

pub fn valid_filename(name: &str) -> bool {
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    (1..=8).contains(&stem.len())
        && extension.eq_ignore_ascii_case("grf")
        && stem.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

pub fn filename(effect_id: &str) -> Result<String> {
    ensure!(valid_id(effect_id), "Invalid effect ID");
    Ok(format!(
        "{}.GRF",
        hex::encode(Sha256::digest(effect_id.as_bytes()))[..8].to_ascii_uppercase()
    ))
}

/// Checks an effect ID (`valid_id`) and a name of 1 to `NAME_MAX` printable ASCII characters.
pub fn check_identity(effect_id: &str, name: &str) -> Result<()> {
    ensure!(
        valid_id(effect_id),
        "Expected a 1 to 31 character lowercase ASCII effect ID"
    );
    ensure!(
        (1..=NAME_MAX).contains(&name.len())
            && name.trim() == name
            && name.bytes().all(|c| (32..=126).contains(&c)),
        "Expected 1 to 16 printable ASCII name characters without edge spaces"
    );
    Ok(())
}

fn check_icon(icon: &[u8]) -> Result<()> {
    ensure!(icon.len() == ICON_SIZE, "Expected a 40 x 40 RGBA8 icon");
    ensure!(
        icon.iter()
            .skip(3)
            .step_by(4)
            .all(|alpha| *alpha == 0 || *alpha == 255),
        "Expected binary icon alpha"
    );
    Ok(())
}

pub fn checksum(blob: &[u8]) -> u32 {
    // The CRC field reads as zero.
    let crc = GRF_CRC..GRF_CRC + 4;
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&blob[..blob.len().min(crc.start)]);
    if blob.len() > crc.start {
        hasher.update(&[0; 4][..(blob.len() - crc.start).min(4)]);
    }
    if blob.len() > crc.end {
        hasher.update(&blob[crc.end..]);
    }
    hasher.finalize()
}

/// Encodes an effect in the decoded JSON form (`Effect::from_json`) with `icon`.
pub fn encode(effect: &Value, icon: &[u8]) -> Result<Vec<u8>> {
    Effect::from_json(effect)?.encode(icon)
}

/// Reads one GRFX0001 file: the effect and its icon.
pub fn decode(blob: &[u8]) -> Result<(Effect, Vec<u8>)> {
    ensure!(
        (blob.len() == SIZE || blob.len() == SIZE_WITH_LEVELS) && &blob[..8] == MAGIC,
        "Expected one complete GRFX0001 effect"
    );
    let read_u32 = |offset| u32::from_le_bytes(blob[offset..offset + 4].try_into().unwrap());
    let read_i16 = |offset: usize| i16::from_le_bytes([blob[offset], blob[offset + 1]]);
    ensure!(
        read_u32(GRF_BYTES) == blob.len() as u32 && read_u32(GRF_CRC) == checksum(blob),
        "Invalid effect size or CRC32"
    );
    let features = read_u32(GRF_FEATURES);
    let levels = features & FEATURE_HUE_LEVELS != 0;
    let native = features & FEATURE_NATIVE_LOOK != 0;
    let defaults = features & FEATURE_FN_DEFAULTS != 0;
    ensure!(
        (levels || native || defaults) == (blob.len() == SIZE_WITH_LEVELS)
            && (!levels || features & FEATURE_HUE_SECTORS != 0),
        "Hue levels need hue sectors and the larger effect size"
    );
    ensure!(
        !native || features & (FEATURE_HUE_SECTORS | FEATURE_HUE_LEVELS) == 0,
        "A native look excludes hue sectors"
    );
    let hue_end = if native {
        native_look::HEAD_END
    } else if features & FEATURE_HUE_SECTORS != 0 {
        HUE_OFFSET + hue::SIZE
    } else {
        HUE_OFFSET
    };
    // The bytes from `SIZE` that the features use: the hue levels or the native set,
    // then the Fn defaults.
    let tail = if native {
        native_look::END
    } else if levels {
        SIZE + hue::LEVEL_SIZE
    } else {
        blob.len().min(SIZE)
    };
    let (defaults_start, defaults_end) = if defaults {
        (FN_DEFAULTS, FN_DEFAULTS_END)
    } else {
        (blob.len(), blob.len())
    };
    ensure!(
        [
            (GRF_RESERVED, GRF_MATRIX),
            (GRF_PADDING, GRF_CURVES),
            (hue_end, SIZE),
            (tail, defaults_start),
            (defaults_end, blob.len()),
        ]
        .iter()
        .all(|(start, end)| blob[*start..*end].iter().all(|v| *v == 0)),
        "Unsupported effect flags or reserved bytes"
    );
    let string = |offset| -> Result<&str> {
        let field = &blob[offset..offset + 32];
        let end = field
            .iter()
            .position(|c| *c == 0)
            .context("Expected a terminated, zero-padded ASCII field")?;
        ensure!(
            field[end..].iter().all(|c| *c == 0) && field[..end].is_ascii(),
            "Expected a terminated, zero-padded ASCII field"
        );
        Ok(std::str::from_utf8(&field[..end])?)
    };
    let id = string(GRF_ID)?;
    let name = string(GRF_NAME)?;
    ensure!(features & !FEATURES == 0, "Unsupported effect features");
    let post: [i16; 9] = std::array::from_fn(|i| read_i16(GRF_POST + i * 2));
    // A field without its feature bit must stay zero, so the bytes stay canonical.
    ensure!(
        features & FEATURE_POST != 0 || post.iter().all(|v| *v == 0),
        "Unsupported effect features"
    );
    let selective = &blob[GRF_SELECTIVE..GRF_RESERVED];
    ensure!(
        features & FEATURE_SELECTIVE != 0 || selective.iter().all(|v| *v == 0),
        "Unsupported effect features"
    );
    let effect = Effect {
        id: id.to_owned(),
        name: name.to_owned(),
        flags: read_u32(GRF_FLAGS),
        matrix: std::array::from_fn(|i| read_i16(GRF_MATRIX + i * 2)),
        curves: std::array::from_fn(|c| {
            std::array::from_fn(|i| {
                let offset = GRF_CURVES + c * 512 + i * 2;
                u16::from_le_bytes([blob[offset], blob[offset + 1]])
            })
        }),
        post_matrix: (features & FEATURE_POST != 0).then_some(post),
        selective: (features & FEATURE_SELECTIVE != 0)
            .then(|| Selective::from_bytes(selective))
            .transpose()?,
        hue_sectors: (features & FEATURE_HUE_SECTORS != 0)
            .then(|| {
                hue::HueSectors::from_bytes(
                    &blob[HUE_OFFSET..hue_end],
                    levels.then(|| &blob[SIZE..SIZE + hue::LEVEL_SIZE]),
                )
            })
            .transpose()?,
        native_look: native.then(|| NativeLook::read(blob)).transpose()?,
        fn_defaults: defaults.then(|| std::array::from_fn(|k| blob[FN_DEFAULTS + k] as i8)),
    };
    let icon = blob[GRF_ICON..GRF_ICON + ICON_SIZE].to_vec();
    effect.check()?;
    check_icon(&icon)?;
    Ok((effect, icon))
}

pub fn interpolate_curve(points: &Value) -> Result<Vec<u16>> {
    let error = "Expected increasing curve knots from index 0 to 255";
    let values = points.as_array().context(error)?;
    ensure!(values.len() >= 2, "{error}");
    let mut knots = Vec::with_capacity(values.len());
    for value in values {
        let pair = value.as_array().context(error)?;
        ensure!(pair.len() == 2, "{error}");
        let x = pair[0].as_i64().context(error)?;
        let y = pair[1].as_i64().context(error)?;
        ensure!(
            (0..=255).contains(&x) && (0..=16383).contains(&y),
            "{error}"
        );
        knots.push((x, y));
    }
    ensure!(
        knots[0].0 == 0
            && knots.last().unwrap().0 == 255
            && knots.windows(2).all(|p| p[0].0 < p[1].0),
        "{error}"
    );
    let mut result = Vec::with_capacity(256);
    for pair in knots.windows(2) {
        let ((left, low), (right, high)) = (pair[0], pair[1]);
        for index in left..right {
            // Match Python's integer product, division, then addition without fused arithmetic.
            let value = low as f64 + ((high - low) * (index - left)) as f64 / (right - left) as f64;
            result.push(value.round_ties_even() as u16);
        }
    }
    result.push(knots.last().unwrap().1 as u16);
    Ok(result)
}

pub fn compile_definition(definition: &Value) -> Result<Vec<u8>> {
    let mut required = vec![
        "schema",
        "effect_id",
        "name",
        "flags",
        "matrix",
        "icon",
        "attribution",
    ];
    required.push(if definition.get("curve_knots").is_some() {
        "curve_knots"
    } else {
        "curves"
    });
    ensure!(
        has_keys_with(definition, &required, &EXTENSION_KEYS) && definition["schema"] == SCHEMA,
        "Expected a grkit-effect-v1 definition"
    );
    ensure!(
        definition["attribution"].is_object(),
        "Expected attribution metadata"
    );
    let pixels = icon_pixels(&definition["icon"])?;
    let curves = if let Some(knots) = definition.get("curve_knots") {
        let channels = knots
            .as_array()
            .context("Expected three curve-knot channels")?;
        ensure!(channels.len() == 3, "Expected three curve-knot channels");
        serde_json::to_value(
            channels
                .iter()
                .map(interpolate_curve)
                .collect::<Result<Vec<_>>>()?,
        )?
    } else {
        definition["curves"].clone()
    };
    // A definition writes selective color in degrees.
    let selective = definition
        .get("selective_color")
        .map(Selective::from_definition)
        .transpose()?;
    Effect::read(definition, &curves, selective)?.encode(&pixels)
}
