use super::file::file_records;
use super::layout::*;
use super::presets::NEGATIVE_FILM;
use super::words::glow_of_word;
use super::*;
use crate::abi::RECIPE_WORDS;
use crate::native;

fn index(label: &str) -> usize {
    native::TYPES.iter().position(|t| t.label == label).unwrap()
}

pub(crate) fn sample() -> Vec<Recipe> {
    vec![
        Recipe {
            name: "Portra Days".into(),
            family: Family::Gr3,
            look: Look::Effect {
                code: 0x1234,
                adjustments: [4, 5, 3, 4, 4, 4, 4, 8, 0],
            },
            grain: Some(Grain { level: 2, size: 0 }),
            glow: Some(Glow {
                bloom: 1,
                halation: 3,
                halation_color: Some(2),
            }),
            white_balance: Some(WhiteBalance {
                mode: 4,
                shift_ba: 17,
                shift_gm: 12,
                kelvin: 5500,
            }),
            corrections: [Some(2), None, Some(1), Some(0)],
            exposure: Exposure {
                compensation: Some(-2),
                iso_limit: Some(3200),
                metering: Some(Metering::Spot),
                nd: Some(Nd::Auto),
            },
            enabled: true,
            hidden: false,
            number: 0,
            id: 0,
        },
        Recipe {
            name: "Nega 2".into(),
            family: Family::Gr4,
            look: Look::Native(NativeLook {
                image_type: index("NegativeFilm"),
                levels: [6, 3, 4, 2, 4, 4, 5, 4, 7],
                tone: 0,
                intensity: 0,
                filter: 0,
                grain_strength: 2,
                grain_size: 1,
                solarization: 0,
            }),
            grain: None,
            glow: None,
            white_balance: Some(WhiteBalance {
                mode: 14,
                shift_ba: 14,
                shift_gm: 14,
                kelvin: 3200,
            }),
            corrections: [None; 4],
            exposure: Exposure::default(),
            enabled: true,
            hidden: false,
            number: 0,
            id: 0,
        },
        Recipe {
            name: "Solar".into(),
            family: Family::Gr4Monochrome,
            look: Look::Native(NativeLook {
                image_type: index("MonochromeSolarization"),
                levels: [4, 4, 5, 4, 4, 4, 3, 4, 6],
                tone: 1,
                intensity: 0,
                filter: 0,
                grain_strength: 0,
                grain_size: 0,
                solarization: 2,
            }),
            grain: Some(Grain { level: 4, size: 3 }),
            glow: Some(Glow {
                bloom: 0,
                halation: 2,
                halation_color: None,
            }),
            white_balance: None,
            corrections: [Some(0), Some(4), Some(0), Some(5)],
            exposure: Exposure::default(),
            enabled: true,
            hidden: false,
            number: 0,
            id: 0,
        },
    ]
}

#[test]
fn a_library_round_trips() {
    let recipes = sample();
    let bytes = encode(&recipes).unwrap();
    assert_eq!(bytes.len(), HEADER_SIZE + 3 * RECORD_SIZE);
    assert_eq!(decode(&bytes).unwrap(), recipes);
    // Each word stays below 2^29, as the camera checks.
    for recipe in &recipes {
        assert!(recipe.words().unwrap().iter().all(|&w| w < WORD_LIMIT));
    }
}

#[test]
fn the_words_follow_the_camera_layout() {
    let words = sample()[1].words().unwrap();
    // Film Look Off, the native type + 1, ColorTemp1 + 1, no shift.
    assert_eq!(words[0], USED | LOOK_OFF);
    assert_eq!(words[2] & 31, index("NegativeFilm") as u32 + 1);
    assert_eq!(words[2] >> MODE_SHIFT & 31, 15);
    assert_eq!(words[3] & 1023, 70);
    assert_eq!(words[3] >> NATIVE_GRAIN_SHIFT & 3, 2);
    // Saturation 6 is digit 2, the first and lowest.
    assert_eq!(words[1] % 9, 2);
    let effect = sample()[0].words().unwrap();
    assert_eq!(effect[0] & 3, LOOK_EFFECT);
    assert_eq!(effect[0] >> CODE_SHIFT & CODE_MASK, 0x1234);
    assert_eq!(effect[0] >> GRAIN_LEVEL_SHIFT & 7, 2);
    assert_eq!(effect[3] >> CORRECTION_SHIFT & 7, 3);
    assert_eq!(effect[3] >> (CORRECTION_SHIFT + 3) & 7, 0);
    // Bloom 1, Halation 3, color 2: held bit, then the value.
    assert_eq!(effect[MODULE_WORD] & 0x1ff, 0b101_111_011);
    // -2/3 stop is -4 sixths + 31, ISO 3200 the 16th third stop, Spot 2 + 1,
    // ND Auto 0 + 1.
    assert_eq!(effect[MODULE_WORD] >> EV_SHIFT & 63, 27);
    assert_eq!(effect[MODULE_WORD] >> ISO_SHIFT & 63, 16);
    assert_eq!(effect[MODULE_WORD] >> METERING_SHIFT & 7, 3);
    assert_eq!(effect[MODULE_WORD] >> ND_SHIFT & 3, 1);
    assert_eq!(words[MODULE_WORD], 0);
    // Without a color, the camera keeps its own.
    assert_eq!(sample()[2].words().unwrap()[MODULE_WORD], 0b101_001);
}

#[test]
fn exposure_values_that_the_camera_does_not_have_are_rejected() {
    let mut recipes = sample();
    recipes[0].exposure.compensation = Some(16);
    assert!(recipes[0].check().is_err(), "more than 5 stops");
    let mut recipes = sample();
    recipes[0].exposure.iso_limit = Some(204_800);
    assert!(recipes[0].check().is_err(), "GR III stops at ISO 102400");
    recipes[0].exposure.iso_limit = Some(3000);
    assert!(recipes[0].check().is_err(), "not a third stop");
    recipes[1].exposure.iso_limit = Some(204_800);
    assert!(recipes[1].check().is_ok());
    // A library recipe holds its compensation in thirds.
    assert!(Exposure::of_bits(30 << EV_SHIFT).is_err());
}

#[test]
fn the_recipe_file_round_trips_and_keeps_the_import() {
    let mut recipes = sample()
        .into_iter()
        .filter(|recipe| recipe.family == Family::Gr3)
        .collect::<Vec<_>>();
    let mut camera = recipes[0].clone();
    camera.name = String::new();
    camera.number = 2;
    camera.hidden = true;
    recipes.push(camera);
    let previous = format!(
        "{}\r\n123\r\n991000000\r\n5\r\n",
        crate::runtime::RECIPE_IMPORTED_KEY
    );
    let file = write_recipe_file(&recipes, Family::Gr3, &previous).unwrap();
    let text = file.text;
    assert!(text.contains(&format!(
        "{}\r\n123\r\n",
        crate::runtime::RECIPE_IMPORTED_KEY
    )));
    // The recipes come back with the IDs that the file gave them.
    assert_eq!(read_recipe_file(&text, Family::Gr3), file.recipes);
    let ids: Vec<u32> = file.recipes.iter().map(|recipe| recipe.id).collect();
    assert_eq!(ids, [1, 2]);
    // Recipe 1 has the automatic name and number 2, and is hidden.
    let w0 = recipes[1].words().unwrap()[0];
    assert_eq!(w0 >> NUMBER_SHIFT & 15, 2);
    assert_eq!(w0 >> 28 & 1, 1);
    assert!(text.contains(&format!("991000001\r\n{w0}\r\n")));
    // A package record keeps the automatic name, as the recipe file does.
    assert_eq!(decode(&encode(&recipes).unwrap()).unwrap(), recipes);
}

/// A recipe of the card that the site cannot read stays in the file
/// that the site writes: after the recipes of its on or off, and off
/// past `ON_MAX`.
#[test]
fn a_write_keeps_the_recipes_that_the_site_cannot_read() {
    let gr3 = |recipe: &Recipe| recipe.family == Family::Gr3;
    let recipes: Vec<Recipe> = sample().into_iter().filter(gr3).collect();
    let mut card = recipes.clone();
    card[0].name = "Card".into();
    let written = write_recipe_file(&card, Family::Gr3, "").unwrap();
    let mut previous = written.text;
    // A recipe word with a look that no build has, which is on, and its
    // name "Odd".
    let odd = USED | 3;
    let slot = card.len();
    let names = RECIPE_WORDS * crate::abi::RECIPE_SLOTS;
    let name = u32::from(b'O') | u32::from(b'd') << 7 | u32::from(b'd') << 14;
    for (key, value) in [
        (crate::runtime::recipe_key(slot * RECIPE_WORDS), odd),
        (
            crate::runtime::recipe_key(names + slot * crate::abi::RECIPE_NAME_WORDS),
            name,
        ),
    ] {
        previous.push_str(&format!("{key}\r\n{value}\r\n"));
    }
    assert_eq!(read_recipe_file(&previous, Family::Gr3), written.recipes);
    let file = write_recipe_file(&recipes, Family::Gr3, &previous).unwrap();
    let text = file.text;
    assert_eq!(read_recipe_file(&text, Family::Gr3), file.recipes);
    let records = file_records(&text);
    assert_eq!(records.len(), recipes.len() + 1);
    let last = &records[recipes.len()];
    assert_eq!((last.words[0], last.name[0]), (odd, name));
    assert!(last.on());
    // With `ON_MAX` recipes on, it turns off; with `LIBRARY_MAX` recipes,
    // it does not fit.
    let mut full = vec![recipes[0].clone(); ON_MAX];
    let text = write_recipe_file(&full, Family::Gr3, &previous)
        .unwrap()
        .text;
    assert!(!file_records(&text)[ON_MAX].on());
    full.resize(
        LIBRARY_MAX,
        Recipe {
            enabled: false,
            ..recipes[0].clone()
        },
    );
    assert!(write_recipe_file(&full, Family::Gr3, &previous).is_err());
}

#[test]
fn glow_holds_bloom_and_halation_together() {
    assert!(glow_of_word(0b011).is_err());
    assert!(glow_of_word(0b111_000_000).is_err());
    let mut recipes = sample();
    recipes[2].glow = Some(Glow {
        bloom: 0,
        halation: 2,
        halation_color: Some(1),
    });
    assert!(
        recipes[2].check().is_err(),
        "Monochrome has no Halation color"
    );
    recipes[0].glow = Some(Glow {
        bloom: 4,
        halation: 0,
        halation_color: None,
    });
    assert!(recipes[0].check().is_err());
}

#[test]
fn values_that_the_camera_does_not_have_are_rejected() {
    let mut recipes = sample();
    recipes[0].family = Family::Gr4Monochrome;
    assert!(
        recipes[0].check().is_err(),
        "Monochrome has no white balance"
    );
    let mut recipes = sample();
    let Look::Native(look) = &mut recipes[1].look else {
        unreachable!()
    };
    look.image_type = index("CinemaYellow");
    recipes[1].family = Family::Gr3;
    assert!(recipes[1].check().is_err(), "GR III has no Cinema");
    let mut recipes = sample();
    recipes[2].corrections[0] = Some(3);
    assert!(
        recipes[2].check().is_err(),
        "highlight correction has 3 values"
    );
    let mut recipes = sample();
    recipes[0].name = "A name that is too long".into();
    assert!(recipes[0].check().is_err());
    // An empty name is the automatic name of the camera.
    recipes[0].name.clear();
    assert!(recipes[0].check().is_ok());
    recipes[0].name = "負片".into();
    assert!(
        recipes[0].check().is_err(),
        "the English font draws only ASCII"
    );
    assert!(encode(&vec![sample()[0].clone(); LIBRARY_MAX + 1]).is_err());
}

/// A library holds `LIBRARY_MAX` recipes, at most `ON_MAX` of them on. The
/// recipe file has those that are on first; a recipe that is off keeps
/// its off bit through the file and the package library.
#[test]
fn recipes_that_are_off_follow_those_that_are_on() {
    let mut recipes = vec![sample()[0].clone(); LIBRARY_MAX];
    for (index, recipe) in recipes.iter_mut().enumerate() {
        recipe.name = format!("R{index}");
        recipe.enabled = index >= LIBRARY_MAX - ON_MAX;
    }
    assert!(encode(&recipes).is_ok());
    let text = write_recipe_file(&recipes, Family::Gr3, "").unwrap().text;
    let read = read_recipe_file(&text, Family::Gr3);
    assert_eq!(read.len(), LIBRARY_MAX);
    assert!(read[..ON_MAX].iter().all(|recipe| recipe.enabled));
    assert!(read[ON_MAX..].iter().all(|recipe| !recipe.enabled));
    assert_eq!(read[0].name, format!("R{}", LIBRARY_MAX - ON_MAX));
    assert_eq!(read[ON_MAX].name, "R0");
    assert_eq!(decode(&encode(&recipes).unwrap()).unwrap(), recipes);
    // The off bit is bit 28 of word 4, past the exposure settings.
    assert_eq!(recipes[0].words().unwrap()[MODULE_WORD] >> 28, 1);
    recipes[0].enabled = true;
    let error = encode(&recipes).unwrap_err();
    assert!(format!("{error:#}").contains("At most 40"), "{error:#}");
    assert!(write_recipe_file(&recipes, Family::Gr3, "").is_err());
}

#[test]
fn the_effect_code_is_that_of_the_installer() {
    // FNV-1a of "portra-400" in the low 15 bits.
    let hash = "portra-400".bytes().fold(0x811c_9dc5u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    });
    assert_eq!(u32::from(effect_code("portra-400")), hash & 0x7fff);
}

/// Each preset holds a recipe that the camera can hold, for distinct
/// camera lines, with the code of a built-in effect. A community recipe
/// names its author and its post.
#[test]
fn the_presets_are_recipes_of_the_camera() {
    assert_eq!(native::TYPES[NEGATIVE_FILM].label, "NegativeFilm");
    for preset in presets() {
        assert!(encode(&preset.recipes).is_ok(), "{}", preset.id);
        let families: std::collections::BTreeSet<_> = preset
            .recipes
            .iter()
            .map(|recipe| recipe.family.code())
            .collect();
        assert_eq!(families.len(), preset.recipes.len(), "{}", preset.id);
        assert_eq!(
            preset.author.is_some(),
            preset.source.is_some(),
            "{}",
            preset.id
        );
        assert!(
            preset.source.is_none_or(|url| url.starts_with("https://")),
            "{}",
            preset.id
        );
        for recipe in &preset.recipes {
            // A whole look (runtime/recipes.h): each setting of the picture,
            // so the preset gives the same picture after any other recipe.
            // The exposure settings belong to one scene and stay out.
            assert!(
                matches!(recipe.look, Look::Effect { .. } | Look::Native(_)),
                "{}",
                preset.id
            );
            assert!(recipe.grain.is_some(), "{}", preset.id);
            let glow = recipe
                .glow
                .unwrap_or_else(|| panic!("{} has no Glow", preset.id));
            let color = !recipe.family.monochrome();
            assert_eq!(glow.halation_color.is_some(), color, "{}", preset.id);
            assert_eq!(recipe.white_balance.is_some(), color, "{}", preset.id);
            assert!(
                recipe.corrections.iter().all(Option::is_some),
                "{}",
                preset.id
            );
            assert_eq!(recipe.exposure, Exposure::default(), "{}", preset.id);
            match (&recipe.look, preset.effect) {
                (Look::Effect { code, adjustments }, Some(effect)) => {
                    assert_eq!(*code, effect_code(effect));
                    // A preset keeps the Fn defaults of its effect: a
                    // stored 0 is the default, not a native 0.
                    assert_eq!(*adjustments, [0; 9], "{}", preset.id);
                    assert_eq!(recipe.words().unwrap()[1], 0, "{}", preset.id);
                    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../effects")
                        .join(effect)
                        .join("effect.json");
                    assert!(source.exists(), "{} needs the effect {effect}", preset.id);
                }
                (Look::Effect { .. }, None) => panic!("{} names no effect", preset.id),
                (_, effect) => assert!(effect.is_none(), "{}", preset.id),
            }
        }
    }
}

/// The stored values of the CineStill preset name the settings that its
/// comment gives.
#[test]
fn the_cinestill_preset_holds_its_camera_settings() {
    let label = |labels: &native::Labels, stored: u8| {
        labels
            .values()
            .find(|(_, value)| *value == u32::from(stored))
            .map(|(label, _)| label)
    };
    let preset = presets()
        .into_iter()
        .find(|preset| preset.id == "cinestill-800t")
        .unwrap();
    for recipe in &preset.recipes {
        let balance = recipe.white_balance.unwrap();
        assert_eq!(native::WHITE_BALANCE[balance.mode].labels[0], Some("Auto"));
        assert_eq!(label(&native::SHIFT_BA, balance.shift_ba), Some("B10"));
        assert_eq!(label(&native::SHIFT_GM, balance.shift_gm), Some("NORMAL"));
        let corrections: Vec<_> = recipe
            .corrections
            .iter()
            .zip(&native::CORRECTIONS)
            .map(|(stored, correction)| label(&correction.labels, stored.unwrap()))
            .collect();
        assert_eq!(
            corrections,
            [Some("Auto"), Some("Off"), Some("Off"), Some("Low")]
        );
    }
}

#[test]
fn a_damaged_file_is_rejected() {
    let mut bytes = encode(&sample()).unwrap();
    bytes[HEADER_SIZE + 5] ^= 1;
    assert!(decode(&bytes).is_err());
    let bytes = encode(&sample()).unwrap();
    assert!(decode(&bytes[..bytes.len() - 1]).is_err());
}

/// Two recipes of the sample line GR III that differ in their name.
fn named(names: &[&str]) -> Vec<Recipe> {
    let base = sample()
        .into_iter()
        .find(|recipe| recipe.family == Family::Gr3)
        .unwrap();
    names
        .iter()
        .map(|name| Recipe {
            name: (*name).into(),
            ..base.clone()
        })
        .collect()
}

/// A recipe keeps its ID. A recipe without one, or with the ID of an
/// earlier recipe, gets the next ID, which stays above each ID that the
/// card gave: a deleted recipe's ID does not come back.
#[test]
fn a_write_keeps_each_id_and_gives_new_ones_above_the_old() {
    use crate::runtime::{RECIPE_ID_MAX, RECIPE_NEXT_ID_KEY};
    let first = write_recipe_file(&named(&["A", "B"]), Family::Gr3, "").unwrap();
    let ids = |file: &RecipeFile| file.recipes.iter().map(|r| r.id).collect::<Vec<_>>();
    assert_eq!(ids(&first), [1, 2]);
    assert!(first
        .text
        .contains(&format!("{RECIPE_NEXT_ID_KEY}\r\n3\r\n")));
    // The site deletes A and adds C: B keeps 2, C gets 3, not the free 1.
    let mut next = vec![first.recipes[1].clone()];
    next.extend(named(&["C"]));
    let second = write_recipe_file(&next, Family::Gr3, &first.text).unwrap();
    assert_eq!(ids(&second), [2, 3]);
    // A copy of B with B's ID gets a new one.
    let copy = Recipe {
        name: "B2".into(),
        ..second.recipes[0].clone()
    };
    let third = write_recipe_file(
        &[second.recipes[0].clone(), copy],
        Family::Gr3,
        &second.text,
    )
    .unwrap();
    assert_eq!(ids(&third), [2, 4]);
    // A file without the next ID: the next ID is still above each ID of the
    // file, so C does not get B's 2 after the site deletes B.
    let without_next = first
        .text
        .replace(&format!("{RECIPE_NEXT_ID_KEY}\r\n3\r\n"), "");
    let mut kept_a = vec![first.recipes[0].clone()];
    kept_a.extend(named(&["C"]));
    let fourth = write_recipe_file(&kept_a, Family::Gr3, &without_next).unwrap();
    assert_eq!(ids(&fourth), [1, 3]);
    // An ID out of range is no ID.
    let mut wrong = named(&["D"]);
    wrong[0].id = RECIPE_ID_MAX + 1;
    assert!(wrong[0].check().is_err());
}

/// The user modes of the mode dial stay as the card has them. A user mode
/// whose recipe the write leaves out follows; GRKit Off stays.
#[test]
fn a_write_keeps_the_user_modes_of_the_mode_dial() {
    use crate::runtime::{RECIPE_DIAL_FOLLOW, RECIPE_DIAL_KEY_BASE, RECIPE_DIAL_OFF};
    let card = write_recipe_file(&named(&["A", "B"]), Family::Gr3, "").unwrap();
    // U1 applies B (ID 2), U2 turns GRKit off, U3 applies A (ID 1).
    let mut previous = card.text.clone();
    for (u, value) in [2, RECIPE_DIAL_OFF, 1].into_iter().enumerate() {
        previous.push_str(&format!(
            "{}\r\n{value}\r\n",
            RECIPE_DIAL_KEY_BASE + u as u32
        ));
    }
    let dial = |text: &str| -> Vec<u32> {
        (0..3)
            .map(|u| {
                let key = format!("{}\r\n", RECIPE_DIAL_KEY_BASE + u);
                text.find(&key).map_or(RECIPE_DIAL_FOLLOW, |at| {
                    text[at + key.len()..]
                        .lines()
                        .next()
                        .unwrap()
                        .parse()
                        .unwrap()
                })
            })
            .collect()
    };
    // The site writes the same recipes in another order: nothing changes.
    let swapped = [card.recipes[1].clone(), card.recipes[0].clone()];
    let same = write_recipe_file(&swapped, Family::Gr3, &previous).unwrap();
    assert_eq!(dial(&same.text), [2, RECIPE_DIAL_OFF, 1]);
    // The site deletes A: U3 follows.
    let without = write_recipe_file(&card.recipes[1..], Family::Gr3, &previous).unwrap();
    assert_eq!(
        dial(&without.text),
        [2, RECIPE_DIAL_OFF, RECIPE_DIAL_FOLLOW]
    );
}

/// A package library holds no IDs: INSTALL gives each added recipe its own.
#[test]
fn a_package_library_holds_no_ids() {
    let file = write_recipe_file(&named(&["A", "B"]), Family::Gr3, "").unwrap();
    assert!(file.recipes.iter().all(|recipe| recipe.id != 0));
    let decoded = decode(&encode(&file.recipes).unwrap()).unwrap();
    assert!(decoded.iter().all(|recipe| recipe.id == 0));
    assert_eq!(decoded, named(&["A", "B"]));
}
