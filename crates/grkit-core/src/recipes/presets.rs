//! The recipes that the site offers.

use super::{effect_code, Exposure, Family, Glow, Grain, Look, NativeLook, Recipe, WhiteBalance};
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

/// The recipes that the site offers.
pub fn presets() -> Vec<Preset> {
    // CineStill 800T is Vision3 500T without its anti-halation layer: the
    // film look and a red halation close around the lights, without the
    // wide glow of Bloom.
    let cinestill = |family| Recipe {
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
        white_balance: None,
        corrections: [None; 4],
        exposure: Exposure::default(),
        enabled: true,
        hidden: false,
        number: 0,
    };
    // A community recipe of Negative Film with its nine levels as the camera
    // shows them, -4 to +4, the Auto white balance with its shifts (stored
    // 14 is none), and the highlight and shadow corrections.
    let negative =
        |name: &str, levels: [i8; 9], shift_gm: i8, shift_ba: i8, corrections: [u8; 2]| Recipe {
            name: name.into(),
            family: Family::Gr3,
            look: Look::Native(NativeLook {
                image_type: NEGATIVE_FILM,
                levels: levels.map(|level| (level + 4) as u8),
                tone: 0,
                intensity: 0,
                filter: 0,
                grain_strength: 0,
                grain_size: 0,
                solarization: 0,
            }),
            grain: None,
            glow: None,
            white_balance: Some(WhiteBalance {
                mode: 0,
                shift_ba: (shift_ba + 14) as u8,
                shift_gm: (shift_gm + 14) as u8,
                kelvin: 5500,
            }),
            corrections: [Some(corrections[0]), Some(corrections[1]), None, None],
            exposure: Exposure::default(),
            enabled: true,
            hidden: false,
            number: 0,
        };
    vec![
        // Highlight correction On, shadow correction Off; AWB G2 A14.
        Preset {
            id: "bennet-800",
            effect: None,
            author: Some("Bennet"),
            source: Some("https://www.reddit.com/r/ricohGR/comments/1k27hmk/"),
            recipes: vec![negative(
                "Bennet 800",
                [3, -1, 0, 2, -2, -4, 0, 0, -1],
                -2,
                14,
                [1, 4],
            )],
        },
        // Highlight correction Auto, shadow correction Mid; AWB A6.
        Preset {
            id: "reggie-negative",
            effect: None,
            author: Some("Reggie Ballesteros"),
            source: Some("https://www.reddit.com/r/ricohGR/comments/180a8nt/"),
            recipes: vec![negative(
                "Reggie Negative",
                [2, 0, 0, 3, -4, -1, 1, 0, 0],
                0,
                6,
                [0, 2],
            )],
        },
        Preset {
            id: "cinestill-800t",
            effect: Some("vision3-500t-supra-v1"),
            author: None,
            source: None,
            recipes: vec![cinestill(Family::Gr3), cinestill(Family::Gr4)],
        },
    ]
}

/// The portable index of Negative Film (`native::TYPES`).
pub(super) const NEGATIVE_FILM: usize = 6;
