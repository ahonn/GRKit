//! The recipes that the site offers.

use super::{effect_code, Exposure, Family, Glow, Grain, Look, NativeLook, Recipe, WhiteBalance};
use crate::native;
use serde::Serialize;

/// A recipe that the site offers: its ID, the effect that it uses, and the
/// recipe for each camera line that can hold it. Other recipes come from the
/// camera.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    /// The ID of the effect of the look, which the package must hold.
    pub effect: Option<&'static str>,
    /// The person who made a community recipe, and the post where they
    /// shared it, or `None` for a recipe of GRKit.
    pub author: Option<&'static str>,
    pub source: Option<&'static str>,
    pub recipes: Vec<Recipe>,
}

/// The white balance and the correction settings of a recipe that sets no
/// others: Auto with no shift, highlight and shadow correction Auto,
/// peripheral illumination correction Off, and high-ISO noise reduction
/// Auto. A preset holds each setting of the look (runtime/recipes.h), so it
/// gives the same picture after any other recipe.
const AUTO_WHITE_BALANCE: WhiteBalance = WhiteBalance {
    mode: 0,
    shift_ba: 14,
    shift_gm: 14,
    kelvin: 5500,
};
const NEUTRAL_CORRECTIONS: [&str; 4] = ["Auto", "Auto", "Off", "Auto"];
const GRAIN_OFF: Grain = Grain { level: 0, size: 0 };
const GLOW_OFF: Glow = Glow {
    bloom: 0,
    halation: 0,
    halation_color: Some(0),
};

/// The camera lines of the color recipes: each but GR IV Monochrome.
const COLOR_LINES: &[Family] = &[Family::Gr3, Family::Gr4];

/// B10: stored A-B shifts run from B14 (0) through none (14) to A14 (28).
const CINESTILL_SHIFT_BA: u8 = 4;

/// A community recipe of a native Image Control type, with its settings as
/// its source gives them and the camera menus show them. A setting that the
/// source does not give takes the neutral value of `AUTO_WHITE_BALANCE` and
/// `NEUTRAL_CORRECTIONS`. It has no GRKit Grain or Glow.
struct Native {
    name: &'static str,
    /// The label of the type (`native::TYPES`).
    image_type: &'static str,
    /// The nine levels, -4 to +4, in portable order: saturation, hue, key,
    /// contrast, highlight contrast, shadow contrast, sharpness, shading,
    /// and clarity. A level that the type does not have is 0.
    levels: [i8; 9],
    /// The label of the value of the tone parameter of the type, if it has
    /// one: "OFF" of Toning, or "Mid" of the Cinema tone.
    tone: Option<&'static str>,
    /// The native grain in portable values: strength 0 (Off) to 3 (High),
    /// and size 0 (Small) to 2 (Large). GR III has one size, 0, so a recipe
    /// of both lines has size 0 on GR IV too.
    grain: (u8, u8),
    /// The label of the white balance mode (`native::WHITE_BALANCE`), its
    /// A-B shift (A positive) and G-M shift (M positive) in steps, and the
    /// color temperature of `ColorTemp1`.
    white_balance: (&'static str, i8, i8, u32),
    /// The labels of the highlight correction, the shadow correction, the
    /// peripheral illumination correction, and the high-ISO noise reduction
    /// (`native::CORRECTIONS`).
    corrections: [&'static str; 4],
}

impl Native {
    /// The recipes of the lines.
    fn recipes(&self, lines: &[Family]) -> Vec<Recipe> {
        lines.iter().map(|&family| self.recipe(family)).collect()
    }

    fn recipe(&self, family: Family) -> Recipe {
        let image_type = native::TYPES
            .iter()
            .position(|t| t.label == self.image_type)
            .unwrap_or_else(|| panic!("no type {}", self.image_type));
        let native_family = family.native();
        // The tone parameter of the type in the family: one of the
        // ColorTone parameters.
        let tone = self.tone.map_or(0, |label| {
            let parameter = (0..native::PARAMETERS.len())
                .find(|&p| {
                    native::PARAMETERS[p].key.starts_with("ColorTone")
                        && native::parameter_name(native_family, image_type, p).is_some()
                })
                .unwrap_or_else(|| panic!("{} has no tone", self.image_type));
            let labels = native::type_parameter_labels(native_family, image_type, parameter)
                .unwrap_or_else(|| panic!("{} has no tone labels", self.image_type));
            stored(&labels, label)
        });
        let (mode, ba, gm, kelvin) = self.white_balance;
        let mode = native::WHITE_BALANCE
            .iter()
            .position(|m| m.labels.contains(&Some(mode)))
            .unwrap_or_else(|| panic!("no white balance mode {mode}"));
        Recipe {
            name: self.name.into(),
            family,
            look: Look::Native(NativeLook {
                image_type,
                levels: self.levels.map(|level| (level + 4) as u8),
                tone,
                intensity: 0,
                filter: 0,
                grain_strength: self.grain.0,
                grain_size: self.grain.1,
                solarization: 0,
            }),
            grain: Some(GRAIN_OFF),
            glow: Some(GLOW_OFF),
            // A preset keeps the border of the camera.
            grain_style: None,
            border: None,
            white_balance: Some(WhiteBalance {
                mode,
                shift_ba: (ba + 14) as u8,
                shift_gm: (gm + 14) as u8,
                kelvin,
            }),
            corrections: std::array::from_fn(|n| {
                Some(stored(&native::CORRECTIONS[n].labels, self.corrections[n]))
            }),
            exposure: Exposure::default(),
            enabled: true,
            hidden: false,
            number: 0,
            id: 0,
        }
    }
}

/// The stored value of a label.
fn stored(labels: &native::Labels, label: &str) -> u8 {
    labels
        .values()
        .find(|(name, _)| *name == label)
        .map(|(_, value)| value as u8)
        .unwrap_or_else(|| panic!("no label {label}"))
}

/// The recipes that the site offers.
pub fn presets() -> Vec<Preset> {
    // CineStill 800T is Vision3 500T without its anti-halation layer: the
    // film look and a red halation close around the lights, without the
    // wide glow of Bloom. The film is balanced for tungsten light. Auto white
    // balance shifted to B10 (about 30 mired toward blue) puts night scenes
    // in the colors of CineStill 800T scans and keeps daylight usable, where
    // a fixed 3200 K turns blue. Shadow correction Off keeps the deep
    // shadows, peripheral illumination correction Off keeps the falloff of
    // the lens, and high-ISO noise reduction Low keeps the texture under the
    // Grain.
    let cinestill = |&family: &Family| Recipe {
        name: "CineStill 800T".into(),
        family,
        look: Look::Effect {
            code: effect_code("vision3-500t-supra-v1"),
            adjustments: [0; 9],
        },
        grain: Some(Grain { level: 2, size: 0 }),
        glow: Some(Glow {
            bloom: 0,
            halation: 2,
            halation_color: Some(0),
        }),
        // A preset keeps the border of the camera.
        grain_style: None,
        border: None,
        white_balance: Some(WhiteBalance {
            shift_ba: CINESTILL_SHIFT_BA,
            ..AUTO_WHITE_BALANCE
        }),
        // Highlight Auto, shadow Off, peripheral illumination Off, NR Low.
        corrections: [Some(0), Some(4), Some(0), Some(1)],
        exposure: Exposure::default(),
        enabled: true,
        hidden: false,
        number: 0,
        id: 0,
    };
    // The site lists the presets in this order: the recipe of GRKit, then
    // the community recipes by look: color negative, positive, cinema, and
    // black and white. The community recipes give their settings for GR III
    // or GR IIIx. GR IV has the same types with the same levels, so each one
    // that uses no GR IV setting is on both color lines.
    vec![
        Preset {
            id: "cinestill-800t",
            effect: Some("vision3-500t-supra-v1"),
            author: None,
            source: None,
            recipes: COLOR_LINES.iter().map(cinestill).collect(),
        },
        Preset {
            id: "bennet-800",
            effect: None,
            author: Some("Bennet"),
            source: Some("https://www.reddit.com/r/ricohGR/comments/1k27hmk/"),
            recipes: Native {
                name: "Bennet 800",
                image_type: "NegativeFilm",
                levels: [3, -1, 0, 2, -2, -4, 0, 0, -1],
                tone: None,
                grain: (0, 0),
                white_balance: ("Auto", 14, -2, 5500),
                corrections: ["On", "Off", "Off", "Auto"],
            }
            .recipes(COLOR_LINES),
        },
        Preset {
            id: "reggie-negative",
            effect: None,
            author: Some("Reggie Ballesteros"),
            source: Some("https://www.reddit.com/r/ricohGR/comments/180a8nt/"),
            recipes: Native {
                name: "Reggie Negative",
                image_type: "NegativeFilm",
                levels: [2, 0, 0, 3, -4, -1, 1, 0, 0],
                tone: None,
                grain: (0, 0),
                white_balance: ("Auto", 6, 0, 5500),
                corrections: ["Auto", "Mid", "Off", "Auto"],
            }
            .recipes(COLOR_LINES),
        },
        // The reply of the author to the post gives the settings, with
        // clarity "0 or -4 (for nostalgic look)": the recipe keeps 0.
        Preset {
            id: "portra-160",
            effect: None,
            author: Some("Chrizz Ocampo"),
            source: Some("https://www.facebook.com/groups/363978064347683/posts/1465331294212349/"),
            recipes: Native {
                name: "Portra 160",
                image_type: "NegativeFilm",
                levels: [3, -1, 3, -1, 2, -3, 0, 0, 0],
                tone: None,
                grain: (0, 0),
                white_balance: ("Shade", 3, -3, 5500),
                corrections: NEUTRAL_CORRECTIONS,
            }
            .recipes(COLOR_LINES),
        },
        Preset {
            id: "cotton-color-400",
            effect: None,
            author: Some("Luke Taylor"),
            source: Some(
                "https://www.lukeptaylor.com/blog/cotton-color-400-ricoh-gr-iii-film-recipe",
            ),
            recipes: Native {
                name: "Cotton Color 400",
                image_type: "PositiveFilm",
                levels: [0, 0, 2, 1, -1, -4, -2, 2, -2],
                tone: None,
                grain: (0, 0),
                white_balance: ("Auto", -2, -3, 5500),
                corrections: ["On", "Mid", "Off", "Off"],
            }
            .recipes(COLOR_LINES),
        },
        Preset {
            id: "candy-color-400",
            effect: None,
            author: Some("Luke Taylor"),
            source: Some(
                "https://www.lukeptaylor.com/blog/candy-color-400-ricoh-gr-iii-film-recipe",
            ),
            recipes: Native {
                name: "Candy Color 400",
                image_type: "PositiveFilm",
                levels: [4, -2, 0, 4, 4, 4, 4, 1, 4],
                tone: None,
                grain: (0, 0),
                white_balance: ("ColorTemp1", 0, 2, 5700),
                corrections: ["On", "High", "On", "Off"],
            }
            .recipes(COLOR_LINES),
        },
        // A GR IV recipe: Cinema Yellow. The video gives Toning 2 (Mid) and
        // grain size 1 and strength 1 (Small, Low), and no white balance.
        Preset {
            id: "lanzarote",
            effect: None,
            author: Some("I Am a Camera"),
            source: Some("https://www.youtube.com/watch?v=omAGSJblE-U"),
            recipes: Native {
                name: "Lanzarote",
                image_type: "CinemaYellow",
                levels: [0, -1, 1, 0, -2, -1, 0, 0, 0],
                tone: Some("Mid"),
                grain: (1, 0),
                white_balance: ("Auto", 0, 0, 5500),
                corrections: NEUTRAL_CORRECTIONS,
            }
            .recipes(&[Family::Gr4]),
        },
        // The comment of the author on the post gives the settings. The
        // grain of GR III Grain Effect 2 is Mid.
        Preset {
            id: "bennet-bw800",
            effect: None,
            author: Some("Bennet"),
            source: Some("https://www.reddit.com/r/ricohGR/comments/1t2rmuv/"),
            recipes: Native {
                name: "Bennet BW800",
                image_type: "MonotoneGR",
                levels: [0, 0, -2, 4, -2, -4, -1, 0, 0],
                tone: Some("OFF"),
                grain: (2, 0),
                white_balance: ("Daylight", 14, -2, 5500),
                corrections: ["On", "Mid", "On", "Off"],
            }
            .recipes(COLOR_LINES),
        },
        // GR III Grain Effect 3 is High. The source gives no white balance:
        // the recipe is black and white.
        Preset {
            id: "dante-sisofo-street-bw",
            effect: None,
            author: Some("Dante Sisofo"),
            source: Some("https://dantesisofo.com/ricoh-griii-settings/"),
            recipes: Native {
                name: "Dante Street B&W",
                image_type: "HighContrastBW",
                levels: [0, 0, -2, 4, -4, 0, 4, 4, 4],
                tone: Some("OFF"),
                grain: (3, 0),
                white_balance: ("Auto", 0, 0, 5500),
                corrections: NEUTRAL_CORRECTIONS,
            }
            .recipes(COLOR_LINES),
        },
    ]
}
