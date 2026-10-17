//! Effect files for the website: compiled effects, preview tables, and their catalog.
//!
//! A preview table answers "how would this Standard JPEG look with the effect?". For each
//! sRGB input it undoes the Standard rendering of the fit model (Standard look matrix, then
//! base gamma, no curves) to scene-linear light, then renders that light with the effect.
//! An effect equal to Standard maps each input to itself.

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

use crate::cube::{self, Matrix};
use crate::fit::{self, Model, Params};
use crate::selective::{self, Selective};
use crate::{effects, packaging, Files};

/// Points on each axis of a preview table.
pub const LUT_SIZE: usize = 33;
pub const CATALOG_SCHEMA: &str = "grkit-site-effects-v1";
/// A bound on the catalog. A package still holds at most `runtime::MAX_COUNT` effects.
pub const MAX_EFFECTS: usize = 100;

fn unit(m: &[i16; 9]) -> [f64; 9] {
    m.map(|v| v as f64 / 8192.0)
}

fn coefficients<const N: usize>(value: &Value, what: &str) -> Result<[i64; N]> {
    let values = value
        .as_array()
        .filter(|v| v.len() == N)
        .with_context(|| format!("Expected {N} {what} values"))?;
    let mut result = [0; N];
    for (slot, value) in result.iter_mut().zip(values) {
        *slot = value
            .as_i64()
            .with_context(|| format!("Expected integer {what} values"))?;
    }
    Ok(result)
}

/// The model parameters of a decoded effect.
pub fn params(effect: &Value) -> Result<Params> {
    let q13 = |value: &Value| -> Result<[f64; 9]> {
        Ok(unit(&coefficients::<9>(value, "matrix")?.map(|v| v as i16)))
    };
    let curves = effect["curves"]
        .as_array()
        .filter(|c| c.len() == 3)
        .context("Expected three curves")?;
    let mut result = Params {
        matrix: q13(&effect["matrix"])?,
        curves: [[0.0; 256]; 3],
        post: fit::IDENTITY,
    };
    for (slot, curve) in result.curves.iter_mut().zip(curves) {
        *slot = coefficients::<256>(curve, "curve")?.map(|v| v as f64);
    }
    if let Some(post) = effect.get("post_matrix") {
        result.post = q13(post)?;
    }
    Ok(result)
}

/// The base-gamma index whose output is `code` (0..16384), clipped to the table domain.
fn inverse_base_gamma(base: &[u16; 256], code: f64) -> f64 {
    let base = base.map(f64::from);
    if code <= base[0] {
        return 0.0;
    }
    let Some(upper) = (1..256).find(|&i| base[i] >= code) else {
        return 255.0;
    };
    let (low, high) = (base[upper - 1], base[upper]);
    upper as f64 - 1.0 + (code - low) / (high - low)
}

/// Inverts the Standard rendering, so `Model::render` with Standard returns the input again.
struct Standard {
    base: [u16; 256],
    inverse: Matrix,
}

impl Standard {
    fn new(base: &[u16; 256]) -> Self {
        let look = unit(&fit::STANDARD_LOOK);
        let rows: Matrix = std::array::from_fn(|r| std::array::from_fn(|c| look[r * 3 + c]));
        Self {
            base: *base,
            inverse: cube::inverse(&rows).expect("the Standard look matrix is invertible"),
        }
    }

    /// Scene-linear light that Standard renders as the sRGB-encoded `display`.
    fn linear(&self, display: [f64; 3]) -> [f64; 3] {
        let mixed = display.map(|v| inverse_base_gamma(&self.base, v * 16384.0) / fit::K);
        cube::mul(&self.inverse, mixed)
    }
}

/// An 8-bit sRGB table with `LUT_SIZE` points on each axis, red fastest, then green, then blue.
pub fn preview_lut(base_gamma: &[u16; 256], effect: &Value) -> Result<Vec<u8>> {
    let params = params(effect)?;
    let selective = effect
        .get("selective_color")
        .map(|value| Selective::from_json(value).map(|s| s.table()))
        .transpose()?;
    let model = Model::new(base_gamma);
    let standard = Standard::new(base_gamma);
    let step = |i: usize| i as f64 / (LUT_SIZE - 1) as f64;
    let mut table = Vec::with_capacity(LUT_SIZE.pow(3) * 3);
    for b in 0..LUT_SIZE {
        for g in 0..LUT_SIZE {
            for r in 0..LUT_SIZE {
                let linear = standard.linear([step(r), step(g), step(b)]);
                let mut display = model.render(&params, linear);
                if let Some(table) = &selective {
                    display = selective::apply_rgb(table, display);
                }
                table.extend(display.map(|v| (v * 255.0).round() as u8));
            }
        }
    }
    Ok(table)
}

fn text<'a>(value: &'a Value, what: &str) -> Result<&'a str> {
    value
        .as_str()
        .with_context(|| format!("Expected effect {what}"))
}

/// Compiles effect definitions for the site catalog of `target`, whose firmware holds
/// `base_gamma`. The result holds `catalog.json`, one `.GRF` per effect under its
/// package filename, and `preview/<id>.lut`.
pub fn publish_effects(
    definitions: &[Value],
    base_gamma: &[u16; 256],
    target: &str,
) -> Result<Files> {
    ensure!(
        (1..=MAX_EFFECTS).contains(&definitions.len()),
        "Expected 1 to {MAX_EFFECTS} effects"
    );
    let mut files = Files::new();
    let mut ids = BTreeSet::new();
    let mut entries = Vec::new();
    for definition in definitions {
        let blob = effects::compile_definition(definition)?;
        let (effect, _) = effects::decode(&blob)?;
        let id = text(&effect["effect_id"], "ID")?;
        let file = effects::filename(id)?;
        ensure!(
            ids.insert(id.to_owned()) && !files.contains_key(&file),
            "Duplicate effect ID or filename hash collision: {id}"
        );
        let attribution = &definition["attribution"];
        let preview = format!("preview/{id}.lut");
        entries.push(json!({
            "id": id,
            "name": effect["name"],
            "label": definition["icon"]["label"],
            "shape": definition["icon"]["shape"],
            "monochrome": effect["flags"].as_u64() == Some(effects::MONOCHROME.into()),
            "look": text(&attribution["target"], "target look")?,
            "origin": text(&attribution["origin"], "origin")?,
            "license": text(&attribution["license"], "license")?,
            "file": file,
            "sha256": packaging::sha256(&blob),
            "preview": preview,
        }));
        files.insert(preview, preview_lut(base_gamma, &effect)?);
        files.insert(file, blob);
    }
    let catalog = json!({
        "schema": CATALOG_SCHEMA,
        "target": target,
        "preview": {"size": LUT_SIZE, "order": "rgb", "input": "standard-srgb8"},
        "effects": entries,
    });
    files.insert("catalog.json".into(), packaging::json_bytes(&catalog)?);
    Ok(files)
}
