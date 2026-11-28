//! The words for the settings of a recipe, in the app language. The values
//! are the stored values of `grkit_core::recipes` and `grkit_core::native`.

use grkit_core::native::{CORRECTIONS, ISO_STEPS};
use grkit_core::recipes::{Exposure, Metering, Nd, Recipe};

use crate::strings::tr;
use crate::ui::setting_icons as icons;

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

/// What a setting of a recipe does to the camera's setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The recipe sets this value.
    Set,
    /// The camera keeps its own value.
    Keep,
    /// The recipe turns the setting off.
    Off,
    /// The recipe sets the setting to 0.
    Zero,
}

/// The picture at the start of a setting's row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// The recipe's Film Look or Image Control type: the page draws it.
    Look,
    /// The pixel icon of a GRKit module, by its ID (`module.toml`: GRAIN).
    Module(&'static str),
    /// The camera's icon of the setting (`crate::ui::setting_icons`).
    Glyph(&'static str),
}

/// One setting of a recipe: its name, its value in words, what it does,
/// and its icon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setting {
    pub name: String,
    pub value: String,
    pub state: State,
    pub icon: Icon,
}

impl Setting {
    fn new(name: String, value: String, state: State, icon: Icon) -> Self {
        Setting {
            name,
            value,
            state,
            icon,
        }
    }

    /// A setting that the camera keeps, with `keep` the words for it.
    fn keep(name: String, keep: &str, icon: Icon) -> Self {
        Setting::new(name, keep.to_owned(), State::Keep, icon)
    }
}

/// A section of settings: its title and its settings.
pub type Group = (String, Vec<Setting>);

/// The icons of the Image Control levels, in the order of `recipes.level.*`.
const LEVEL_ICONS: [&str; 9] = [
    icons::SATURATION,
    icons::HUE,
    icons::KEY,
    icons::CONTRAST,
    icons::CONTRAST_HIGHLIGHT,
    icons::CONTRAST_SHADOW,
    icons::SHARPNESS,
    icons::SHADING,
    icons::CLARITY,
];

/// The icons of the corrections, in the order of `recipes.correction.*`.
const CORRECTION_ICONS: [&str; 4] = [
    icons::HIGHLIGHT,
    icons::SHADOW,
    icons::PERIPHERAL,
    icons::NOISE,
];

/// A level that turns the setting off at 0.
fn level(value: u8) -> State {
    if value == 0 { State::Off } else { State::Set }
}

/// The settings of a recipe, after the setting `look` of its Film Look or
/// Image Control type.
pub fn groups(recipe: &Recipe, look: Setting) -> Vec<Group> {
    groups_in(recipe, look, &|key| tr(key, &[]))
}

/// [`groups`] with `t` for the words of each key, so that a test reads the
/// keys themselves.
fn groups_in(recipe: &Recipe, look: Setting, t: &dyn Fn(&str) -> String) -> Vec<Group> {
    let keep = t("recipes.keep");
    let mut style = vec![look];
    if let grkit_core::recipes::Look::Native(native) = &recipe.look {
        let mono = crate::domain::recipes::monochrome_type(native.image_type);
        for (i, value) in native.levels.iter().enumerate() {
            if mono && i < 2 {
                continue;
            }
            let state = if *value == 4 { State::Zero } else { State::Set };
            style.push(Setting::new(
                t(&format!("recipes.level.{i}")),
                signed(*value, 4),
                state,
                Icon::Glyph(LEVEL_ICONS[i.min(LEVEL_ICONS.len() - 1)]),
            ));
        }
        style.push(Setting::new(
            t("recipes.native_grain"),
            t(&format!("recipes.glow.{}", native.grain_strength.min(3))),
            level(native.grain_strength),
            Icon::Glyph(icons::NATIVE_GRAIN),
        ));
    }
    let grain = Icon::Module("GRAIN");
    style.push(match recipe.grain {
        Some(g) => Setting::new(
            t("recipes.grain"),
            t(&format!("recipes.grain.{}", g.level.min(4))),
            level(g.level),
            grain,
        ),
        None => Setting::keep(t("recipes.grain"), &keep, grain),
    });
    let glow = Icon::Module("GLOW");
    match recipe.glow {
        Some(g) => {
            style.push(Setting::new(
                t("recipes.bloom"),
                t(&format!("recipes.glow.{}", g.bloom.min(3))),
                level(g.bloom),
                glow,
            ));
            let mut halation = t(&format!("recipes.glow.{}", g.halation.min(3)));
            if let (Some(color), true) = (g.halation_color, g.halation > 0) {
                halation = format!(
                    "{halation} · {}",
                    t(&format!("recipes.color.{}", color.min(3)))
                );
            }
            style.push(Setting::new(
                t("recipes.halation"),
                halation,
                level(g.halation),
                glow,
            ));
        }
        None => style.push(Setting::keep(t("recipes.halation"), &keep, glow)),
    }
    let border = Icon::Module("BORDER");
    style.push(match recipe.border {
        Some(b) => {
            let name = t(&format!("recipes.border.{}", b.style.min(3)));
            let value = match b.color.filter(|_| b.style > 0) {
                Some(c) => format!(
                    "{name} · {}",
                    t(&format!("recipes.border_color.{}", c.min(1)))
                ),
                None => name,
            };
            Setting::new(t("recipes.border"), value, level(b.style), border)
        }
        None => Setting::keep(t("recipes.border"), &keep, border),
    });
    let mut groups = vec![(t("recipes.style"), style)];
    let wb = match recipe.white_balance {
        Some(wb) => {
            let shift = |name: &str, value: u8| {
                let state = if value == 14 { State::Zero } else { State::Set };
                Setting::new(
                    name.to_owned(),
                    signed(value, 14),
                    state,
                    Icon::Glyph(icons::WB_SHIFT),
                )
            };
            vec![
                Setting::new(
                    t("recipes.wb_mode"),
                    t(&format!("recipes.wb.{}", wb.mode.min(17))),
                    State::Set,
                    Icon::Glyph(icons::WHITE_BALANCE),
                ),
                shift("A/B", wb.shift_ba),
                shift("G/M", wb.shift_gm),
            ]
        }
        None => vec![Setting::keep(
            t("recipes.wb_mode"),
            &keep,
            Icon::Glyph(icons::WHITE_BALANCE),
        )],
    };
    groups.push((t("recipes.white_balance"), wb));
    let corrections = recipe
        .corrections
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let name = t(&format!("recipes.correction.{i}"));
            let icon = Icon::Glyph(CORRECTION_ICONS[i.min(CORRECTION_ICONS.len() - 1)]);
            let Some(v) = value else {
                return Setting::keep(name, &keep, icon);
            };
            let label = CORRECTIONS[i]
                .labels
                .values()
                .find(|(_, stored)| *stored == u32::from(*v))
                .map(|(label, _)| label);
            let state = if label == Some("Off") {
                State::Off
            } else {
                State::Set
            };
            let value = label
                .map(|label| t(&format!("recipes.value.{label}")))
                .unwrap_or_else(|| v.to_string());
            Setting::new(name, value, state, icon)
        })
        .collect();
    groups.push((t("recipes.corrections"), corrections));
    groups.push((t("recipes.exposure"), exposure(&recipe.exposure, t, &keep)));
    groups
}

fn exposure(e: &Exposure, t: &dyn Fn(&str) -> String, keep: &str) -> Vec<Setting> {
    let compensation = t("recipes.compensation");
    let compensation_icon = Icon::Glyph(icons::EXPOSURE);
    let iso = t("recipes.iso_limit");
    let iso_icon = Icon::Glyph(icons::ISO);
    let metering = t("recipes.metering");
    let metering_icon = Icon::Glyph(icons::METERING);
    let nd = t("recipes.nd");
    let nd_icon = Icon::Glyph(icons::ND);
    vec![
        match e.compensation {
            Some(0) => Setting::new(compensation, "0".into(), State::Zero, compensation_icon),
            Some(thirds) => Setting::new(
                compensation,
                format!("{:+.1}", f32::from(thirds) / 3.0),
                State::Set,
                compensation_icon,
            ),
            None => Setting::keep(compensation, keep, compensation_icon),
        },
        match e.iso_limit {
            Some(limit) => Setting::new(
                iso,
                ISO_STEPS
                    .iter()
                    .find(|s| **s == limit)
                    .copied()
                    .unwrap_or(limit)
                    .to_string(),
                State::Set,
                iso_icon,
            ),
            None => Setting::keep(iso, keep, iso_icon),
        },
        match e.metering {
            Some(m) => {
                let key = match m {
                    Metering::Multi => "multi",
                    Metering::Center => "center",
                    Metering::Spot => "spot",
                    Metering::Highlight => "highlight",
                };
                Setting::new(
                    metering,
                    t(&format!("recipes.metering.{key}")),
                    State::Set,
                    metering_icon,
                )
            }
            None => Setting::keep(metering, keep, metering_icon),
        },
        match e.nd {
            Some(value) => {
                let (key, state) = match value {
                    Nd::Auto => ("recipes.value.Auto", State::Set),
                    Nd::On => ("recipes.value.On", State::Set),
                    Nd::Off => ("recipes.value.Off", State::Off),
                };
                Setting::new(nd, t(key), state, nd_icon)
            }
            None => Setting::keep(nd, keep, nd_icon),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The CineStill 800T preset: Grain, Halation, white balance and two
    /// corrections set, Bloom and two corrections off, G/M at 0, and the
    /// border and exposure left to the camera.
    #[test]
    fn each_setting_says_what_it_does_to_the_camera() {
        let presets = crate::domain::recipes::PresetItem::for_family(
            &grkit_core::release_index::IndexPreset::all(&Default::default()),
            grkit_core::recipes::Family::Gr3,
        );
        let recipe = presets
            .into_iter()
            .find(|p| p.id == "cinestill-800t")
            .expect("bundled preset")
            .recipe;
        let look = Setting::new("look".into(), "Vision3".into(), State::Set, Icon::Look);
        let settings: Vec<Setting> = groups_in(&recipe, look, &|key| key.to_owned())
            .into_iter()
            .flat_map(|(_, settings)| settings)
            .collect();
        let state = |name: String| {
            settings
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.state)
                .unwrap_or_else(|| panic!("no setting {name}"))
        };
        assert_eq!(state("recipes.grain".into()), State::Set);
        assert_eq!(state("recipes.halation".into()), State::Set);
        assert_eq!(state("recipes.bloom".into()), State::Off);
        assert_eq!(state("recipes.border".into()), State::Keep);
        assert_eq!(state("G/M".into()), State::Zero);
        assert_eq!(state("recipes.correction.1".into()), State::Off);
        assert_eq!(state("recipes.compensation".into()), State::Keep);
    }
}
