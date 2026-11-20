//! The words for the settings of a recipe, in the app language. The values
//! are the stored values of `grkit_core::recipes` and `grkit_core::native`.

use grkit_core::native::{CORRECTIONS, ISO_STEPS};
use grkit_core::recipes::{Exposure, Metering, Nd, Recipe};

use crate::strings::tr;

/// A level of GRKit Grain: 0 Off to 4 Extra Strong.
pub fn grain_level(level: u8) -> String {
    tr(&format!("recipes.grain.{}", level.min(4)), &[])
}

/// A level of Bloom, Halation, or native grain: 0 Off to 3 Strong.
pub fn glow_level(level: u8) -> String {
    tr(&format!("recipes.glow.{}", level.min(3)), &[])
}

/// A stored value with 0 in the middle of `values`, signed.
pub fn signed(value: u8, zero: u8) -> String {
    let n = i32::from(value) - i32::from(zero);
    if n > 0 {
        format!("+{n}")
    } else {
        n.to_string()
    }
}

fn keep() -> String {
    tr("recipes.keep", &[])
}

/// A section of rows: its title, and the title and value of each row.
pub type Group = (String, Vec<(String, String)>);

/// The rows of a recipe, after the look row of `look`.
pub fn groups(recipe: &Recipe, look: (String, String)) -> Vec<Group> {
    let mut style = vec![look];
    if let grkit_core::recipes::Look::Native(native) = &recipe.look {
        let mono = crate::domain::recipes::monochrome_type(native.image_type);
        for (i, value) in native.levels.iter().enumerate() {
            if mono && i < 2 {
                continue;
            }
            style.push((tr(&format!("recipes.level.{i}"), &[]), signed(*value, 4)));
        }
        style.push((
            tr("recipes.native_grain", &[]),
            glow_level(native.grain_strength),
        ));
    }
    style.push((
        tr("recipes.grain", &[]),
        recipe.grain.map_or_else(keep, |g| grain_level(g.level)),
    ));
    match recipe.glow {
        Some(glow) => {
            style.push((tr("recipes.bloom", &[]), glow_level(glow.bloom)));
            let mut halation = glow_level(glow.halation);
            if let (Some(color), true) = (glow.halation_color, glow.halation > 0) {
                halation = format!(
                    "{halation} · {}",
                    tr(&format!("recipes.color.{}", color.min(3)), &[])
                );
            }
            style.push((tr("recipes.halation", &[]), halation));
        }
        None => style.push((tr("recipes.halation", &[]), keep())),
    }
    style.push((
        tr("recipes.border", &[]),
        recipe.border.map_or_else(keep, |b| {
            let style = tr(&format!("recipes.border.{}", b.style.min(3)), &[]);
            match b.color.filter(|_| b.style > 0) {
                Some(c) => format!(
                    "{style} · {}",
                    tr(&format!("recipes.border_color.{}", c.min(1)), &[])
                ),
                None => style,
            }
        }),
    ));
    let mut groups = vec![(tr("recipes.style", &[]), style)];
    let wb = match recipe.white_balance {
        Some(wb) => vec![
            (
                tr("recipes.wb_mode", &[]),
                tr(&format!("recipes.wb.{}", wb.mode.min(17)), &[]),
            ),
            ("A/B".to_owned(), signed(wb.shift_ba, 14)),
            ("G/M".to_owned(), signed(wb.shift_gm, 14)),
        ],
        None => vec![(tr("recipes.wb_mode", &[]), keep())],
    };
    groups.push((tr("recipes.white_balance", &[]), wb));
    let corrections = recipe
        .corrections
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let value = value.map_or_else(keep, |v| {
                CORRECTIONS[i]
                    .labels
                    .values()
                    .find(|(_, stored)| *stored == u32::from(v))
                    .map(|(label, _)| tr(&format!("recipes.value.{label}"), &[]))
                    .unwrap_or_else(|| v.to_string())
            });
            (tr(&format!("recipes.correction.{i}"), &[]), value)
        })
        .collect();
    groups.push((tr("recipes.corrections", &[]), corrections));
    groups.push((tr("recipes.exposure", &[]), exposure(&recipe.exposure)));
    groups
}

fn exposure(e: &Exposure) -> Vec<(String, String)> {
    vec![
        (
            tr("recipes.compensation", &[]),
            e.compensation.map_or_else(keep, |t| {
                if t == 0 {
                    "0".into()
                } else {
                    format!("{:+.1}", f32::from(t) / 3.0)
                }
            }),
        ),
        (
            tr("recipes.iso_limit", &[]),
            e.iso_limit.map_or_else(keep, |iso| {
                ISO_STEPS
                    .iter()
                    .find(|s| **s == iso)
                    .copied()
                    .unwrap_or(iso)
                    .to_string()
            }),
        ),
        (
            tr("recipes.metering", &[]),
            e.metering.map_or_else(keep, |m| {
                let key = match m {
                    Metering::Multi => "multi",
                    Metering::Center => "center",
                    Metering::Spot => "spot",
                    Metering::Highlight => "highlight",
                };
                tr(&format!("recipes.metering.{key}"), &[])
            }),
        ),
        (
            tr("recipes.nd", &[]),
            e.nd.map_or_else(keep, |nd| {
                tr(
                    match nd {
                        Nd::Auto => "recipes.value.Auto",
                        Nd::On => "recipes.value.On",
                        Nd::Off => "recipes.value.Off",
                    },
                    &[],
                )
            }),
        ),
    ]
}
