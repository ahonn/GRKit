//! Data-only Image Control definitions and GRFX0001 encoding.
//!
//! Bytes 84..128 hold the optional extension: u32 features, an i16[9] signed Q13
//! post-curve matrix at 88, and the selective color parameters at 106 (see
//! `selective`). Bytes 120..128 are reserved. An effect without features has zeros in
//! the whole extension.

use anyhow::{ensure, Context, Result};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fmt;

use crate::selective::{self, Selective};

pub const MAGIC: &[u8; 8] = b"GRFX0001";
pub const SIZE: usize = 8192;
pub const ICON_SIZE: usize = 6400;
pub const ICON_OFFSET: usize = 1684;
pub const MONOCHROME: u32 = 1;
pub const SCHEMA: &str = "grkit-effect-v1";
pub const FEATURE_POST: u32 = 1;
pub const FEATURE_SELECTIVE: u32 = 2;
const FEATURES: u32 = FEATURE_POST | FEATURE_SELECTIVE;
/// Optional keys of both a definition and a decoded effect. A definition writes
/// `selective_color` in degrees; a decoded effect holds its GRF integers.
const EXTENSION_KEYS: [&str; 2] = ["post_matrix", "selective_color"];
const SELECTIVE_OFFSET: usize = 106;
const RESERVED: std::ops::Range<usize> = SELECTIVE_OFFSET + selective::SIZE..128;

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

/// `required` keys, any of the `optional` keys, and nothing else.
fn has_keys_with(value: &Value, required: &[&str], optional: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        required.iter().all(|k| object.contains_key(*k))
            && object
                .keys()
                .all(|k| required.contains(&k.as_str()) || optional.contains(&k.as_str()))
    })
}

/// The optional extension of an effect: features, the post matrix, and selective color.
struct Extension {
    features: u32,
    post: [i16; 9],
    selective: Option<Selective>,
}

fn extension(effect: &Value) -> Result<Extension> {
    let mut result = Extension {
        features: 0,
        post: [0; 9],
        selective: None,
    };
    if let Some(matrix) = effect.get("post_matrix") {
        let values = matrix
            .as_array()
            .filter(|m| m.len() == 9)
            .context("Expected nine signed Q13 post-matrix coefficients")?;
        for (slot, value) in result.post.iter_mut().zip(values) {
            *slot = value
                .as_i64()
                .and_then(|n| i16::try_from(n).ok())
                .context("Expected nine signed Q13 post-matrix coefficients")?;
        }
        result.features |= FEATURE_POST;
    }
    if let Some(value) = effect.get("selective_color") {
        result.selective = Some(Selective::from_json(value)?);
        result.features |= FEATURE_SELECTIVE;
    }
    Ok(result)
}

pub fn valid_id(id: &str) -> bool {
    (1..=31).contains(&id.len())
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

pub fn coefficients(effect: &Value) -> Result<Vec<u8>> {
    let matrix = effect["matrix"]
        .as_array()
        .context("Expected matrix and curve arrays")?;
    let curves = effect["curves"]
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
    let mut result = Vec::with_capacity(1556);
    for value in matrix {
        result.extend_from_slice(&(value.as_i64().unwrap() as i16).to_le_bytes());
    }
    result.extend_from_slice(&[0, 0]);
    for curve in curves {
        for value in curve.as_array().unwrap() {
            result.extend_from_slice(&(value.as_u64().unwrap() as u16).to_le_bytes());
        }
    }
    Ok(result)
}

pub fn validate(effect: &Value, icon: &[u8]) -> Result<()> {
    let mut optional = vec!["flags"];
    optional.extend(EXTENSION_KEYS);
    ensure!(
        has_keys_with(
            effect,
            &["effect_id", "name", "matrix", "curves"],
            &optional
        ),
        "Expected effect_id, name, matrix, curves, and optional flags, post_matrix, and selective_color"
    );
    let flags = effect
        .get("flags")
        .map_or(Some(0), Value::as_u64)
        .context("Unsupported effect flags")?;
    ensure!(flags <= MONOCHROME as u64, "Unsupported effect flags");
    ensure!(
        effect["effect_id"].as_str().is_some_and(valid_id),
        "Expected a 1 to 31 character lowercase ASCII effect ID"
    );
    let name = effect["name"]
        .as_str()
        .context("Expected an ASCII effect name")?;
    ensure!(
        (1..=16).contains(&name.len())
            && name.trim() == name
            && name.bytes().all(|c| (32..=126).contains(&c)),
        "Expected 1 to 16 printable ASCII name characters without edge spaces"
    );
    coefficients(effect)?;
    let extension = extension(effect)?;
    ensure!(
        flags != MONOCHROME as u64 || extension.features & FEATURE_POST == 0,
        "Monochrome effects cannot have a post matrix"
    );
    ensure!(
        flags != MONOCHROME as u64 || extension.features & FEATURE_SELECTIVE == 0,
        "Monochrome effects cannot have selective color"
    );
    if flags == MONOCHROME as u64 {
        let matrix = effect["matrix"].as_array().unwrap();
        let curves = effect["curves"].as_array().unwrap();
        ensure!(
            matrix[..3] == matrix[3..6]
                && matrix[..3] == matrix[6..9]
                && curves[0] == curves[1]
                && curves[0] == curves[2],
            "Monochrome requires equal matrix rows and RGB curves"
        );
    }
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
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&blob[..blob.len().min(12)]);
    if blob.len() > 12 {
        hasher.update(&[0; 4][..(blob.len() - 12).min(4)]);
    }
    if blob.len() > 16 {
        hasher.update(&blob[16..]);
    }
    hasher.finalize()
}

pub fn encode(effect: &Value, icon: &[u8]) -> Result<Vec<u8>> {
    validate(effect, icon)?;
    let extension = extension(effect)?;
    let mut blob = vec![0; SIZE];
    blob[..8].copy_from_slice(MAGIC);
    blob[84..88].copy_from_slice(&extension.features.to_le_bytes());
    for (index, value) in extension.post.iter().enumerate() {
        blob[88 + index * 2..90 + index * 2].copy_from_slice(&value.to_le_bytes());
    }
    if let Some(selective) = extension.selective {
        blob[SELECTIVE_OFFSET..RESERVED.start].copy_from_slice(&selective.to_bytes());
    }
    blob[8..12].copy_from_slice(&(SIZE as u32).to_le_bytes());
    blob[80..84].copy_from_slice(
        &(effect.get("flags").and_then(Value::as_u64).unwrap_or(0) as u32).to_le_bytes(),
    );
    for (offset, field) in [(16, "effect_id"), (48, "name")] {
        let bytes = effect[field].as_str().unwrap().as_bytes();
        blob[offset..offset + bytes.len()].copy_from_slice(bytes);
    }
    blob[128..ICON_OFFSET].copy_from_slice(&coefficients(effect)?);
    blob[ICON_OFFSET..ICON_OFFSET + ICON_SIZE].copy_from_slice(icon);
    let crc = checksum(&blob);
    blob[12..16].copy_from_slice(&crc.to_le_bytes());
    Ok(blob)
}

pub fn decode(blob: &[u8]) -> Result<(Value, Vec<u8>)> {
    ensure!(
        blob.len() == SIZE && &blob[..8] == MAGIC,
        "Expected one complete GRFX0001 effect"
    );
    let read_u32 = |offset| u32::from_le_bytes(blob[offset..offset + 4].try_into().unwrap());
    ensure!(
        read_u32(8) == SIZE as u32 && read_u32(12) == checksum(blob),
        "Invalid effect size or CRC32"
    );
    ensure!(
        [(RESERVED.start, RESERVED.end), (146, 148), (8084, SIZE)]
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
    let matrix: Vec<i16> = blob[128..146]
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes(bytes.try_into().unwrap()))
        .collect();
    let curves: Vec<Vec<u16>> = blob[148..ICON_OFFSET]
        .chunks_exact(512)
        .map(|channel| {
            channel
                .chunks_exact(2)
                .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap()))
                .collect()
        })
        .collect();
    let mut effect = json!({"effect_id": string(16)?, "name": string(48)?, "flags": read_u32(80), "matrix": matrix, "curves": curves});
    {
        let features = read_u32(84);
        ensure!(features & !FEATURES == 0, "Unsupported effect features");
        let post: Vec<i16> = blob[88..106]
            .chunks_exact(2)
            .map(|bytes| i16::from_le_bytes(bytes.try_into().unwrap()))
            .collect();
        // A field without its feature bit must stay zero, so the bytes stay canonical.
        ensure!(
            features & FEATURE_POST != 0 || post.iter().all(|v| *v == 0),
            "Unsupported effect features"
        );
        let selective = &blob[SELECTIVE_OFFSET..RESERVED.start];
        ensure!(
            features & FEATURE_SELECTIVE != 0 || selective.iter().all(|v| *v == 0),
            "Unsupported effect features"
        );
        let object = effect.as_object_mut().unwrap();
        if features & FEATURE_POST != 0 {
            object.insert("post_matrix".into(), json!(post));
        }
        if features & FEATURE_SELECTIVE != 0 {
            object.insert(
                "selective_color".into(),
                Selective::from_bytes(selective)?.to_json(),
            );
        }
    }
    let icon = blob[ICON_OFFSET..ICON_OFFSET + ICON_SIZE].to_vec();
    validate(&effect, &icon)?;
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
    let icon = &definition["icon"];
    ensure!(
        has_keys(icon, &["label", "shape"]),
        "Expected icon label and shape"
    );
    let pixels = crate::artwork::render_tile(
        icon["label"].as_str().context("Expected icon label")?,
        icon["shape"].as_str().context("Expected icon shape")?,
    )?;
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
    let mut effect = json!({"effect_id": definition["effect_id"], "name": definition["name"], "flags": definition["flags"], "matrix": definition["matrix"], "curves": curves});
    let object = effect.as_object_mut().unwrap();
    if let Some(matrix) = definition.get("post_matrix") {
        object.insert("post_matrix".into(), matrix.clone());
    }
    if let Some(value) = definition.get("selective_color") {
        object.insert(
            "selective_color".into(),
            Selective::from_definition(value)?.to_json(),
        );
    }
    encode(&effect, &pixels)
}
