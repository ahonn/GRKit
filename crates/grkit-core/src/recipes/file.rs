//! The recipe file of the card, `GRKIT/RECIPES.TXT`, which the camera and
//! the site read and write.

use super::layout::{MODULE_WORD, OFF, USED};
use super::{check_library, Family, Recipe, LIBRARY_MAX, ON_MAX, WORD_LIMIT};
use crate::abi::{RECIPE_DIAL_POSITIONS, RECIPE_NAME_WORDS, RECIPE_SLOTS, RECIPE_WORDS};
use crate::runtime::{
    recipe_key, RECIPE_DIAL_FOLLOW, RECIPE_DIAL_KEY_BASE, RECIPE_DIAL_OFF, RECIPE_ID_KEY_BASE,
    RECIPE_ID_MAX, RECIPE_IMPORTED_KEY, RECIPE_NEXT_ID_KEY,
};
use anyhow::{ensure, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

/// A record of the recipe file: the words of a recipe, of its name, and its
/// ID.
pub(super) struct FileRecord {
    pub(super) words: [u32; RECIPE_WORDS],
    pub(super) name: [u32; RECIPE_NAME_WORDS],
    pub(super) id: u32,
}

impl FileRecord {
    /// The recipe of the record for a camera of `family`, or an error when
    /// the checks of the family reject it.
    fn recipe(&self, family: Family) -> Result<Recipe> {
        let mut name = String::new();
        for n in 0..crate::abi::RECIPE_NAME_CHARS {
            let c = self.name[n / 4] >> (7 * (n % 4)) & 0x7f;
            if !(0x20..=0x7e).contains(&c) {
                break;
            }
            name.push(c as u8 as char);
        }
        let mut recipe = Recipe::from_words(name.trim().to_string(), family, self.words)?;
        recipe.id = if self.id <= RECIPE_ID_MAX { self.id } else { 0 };
        Ok(recipe)
    }

    pub(super) fn on(&self) -> bool {
        self.words[MODULE_WORD] & OFF == 0
    }
}

/// The records of the recipe file in its text (key and value lines,
/// `runtime::recipe_key`), in their order. A word that is not a recipe word
/// ends the records.
pub(super) fn file_records(text: &str) -> Vec<FileRecord> {
    let values = file_values(text);
    let word = |index: usize| values.get(&recipe_key(index)).copied().unwrap_or(0);
    let names = RECIPE_WORDS * RECIPE_SLOTS;
    (0..RECIPE_SLOTS)
        .map(|slot| FileRecord {
            words: std::array::from_fn(|k| word(slot * RECIPE_WORDS + k)),
            name: std::array::from_fn(|k| word(names + slot * RECIPE_NAME_WORDS + k)),
            id: values
                .get(&(RECIPE_ID_KEY_BASE + slot as u32))
                .copied()
                .unwrap_or(0),
        })
        .take_while(|record| record.words[0] & USED != 0 && record.words[0] < WORD_LIMIT)
        .collect()
}

/// The recipes of the recipe file (`GRKIT/RECIPES.TXT`) in its text, for a
/// camera of `family`, each with its ID. A recipe that the checks of the
/// family reject stays out, and `write_recipe_file` keeps it.
pub fn read_recipe_file(text: &str, family: Family) -> Vec<Recipe> {
    file_records(text)
        .iter()
        .filter_map(|record| record.recipe(family).ok())
        .collect()
}

/// A recipe file that `write_recipe_file` made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeFile {
    /// The text of the file.
    pub text: String,
    /// The recipes that the file holds for the site, in the order that the
    /// write had them, each with its ID.
    pub recipes: Vec<Recipe>,
}

/// The recipe file for `recipes` of a camera of `family`, which replaces the
/// file text `previous`. The recipes are in the order of the camera: those
/// that are on, then those that are off, each in their order.
///
/// The file keeps from `previous` what the site does not edit:
///
/// - The import hash, so that a package that INSTALL imported does not
///   import again.
/// - Each recipe that `read_recipe_file` leaves out, so that a write never
///   deletes a recipe that the site cannot show. Such a recipe goes after
///   the recipes of its on or off, and turns off past `ON_MAX`.
/// - The user modes of the mode dial. A user mode whose recipe the file no
///   longer holds follows (`runtime::RECIPE_DIAL_FOLLOW`).
///
/// A recipe keeps its ID. A recipe without one, or with the ID of an earlier
/// recipe, gets the next ID, which stays above each ID that `previous` gave,
/// so a deleted recipe never gets the references of another.
pub fn write_recipe_file(recipes: &[Recipe], family: Family, previous: &str) -> Result<RecipeFile> {
    check_library(recipes)?;
    let values = file_values(previous);
    let value = |key: u32| values.get(&key).copied().unwrap_or(0);
    let mut kept: Vec<FileRecord> = file_records(previous)
        .into_iter()
        .filter(|record| record.recipe(family).is_err())
        .collect();
    ensure!(
        recipes.len() + kept.len() <= LIBRARY_MAX,
        "The card holds {} recipes that the site cannot show, so at most {} more fit",
        kept.len(),
        LIBRARY_MAX - kept.len()
    );

    // The IDs: the next ID is above each ID of `previous` and of the write.
    let mut written = recipes.to_vec();
    let mut next = value(RECIPE_NEXT_ID_KEY).clamp(1, RECIPE_ID_MAX);
    let held = file_records(previous).into_iter().map(|record| record.id);
    for id in written.iter().map(|recipe| recipe.id).chain(held) {
        if (1..=RECIPE_ID_MAX).contains(&id) && id >= next {
            next = (id + 1).min(RECIPE_ID_MAX);
        }
    }
    let mut taken = BTreeSet::new();
    let mut give = |id: &mut u32| {
        if !(1..=RECIPE_ID_MAX).contains(id) || !taken.insert(*id) {
            *id = next;
            taken.insert(next);
            next = (next + 1).min(RECIPE_ID_MAX);
        }
    };
    for recipe in &mut written {
        give(&mut recipe.id);
    }
    for record in &mut kept {
        give(&mut record.id);
    }

    let mut records = Vec::with_capacity(written.len() + kept.len());
    for recipe in &written {
        let words = recipe
            .words()
            .with_context(|| format!("Recipe {}", recipe.name))?;
        let mut name = [0u32; RECIPE_NAME_WORDS];
        for (n, byte) in recipe.name.bytes().enumerate() {
            name[n / 4] |= u32::from(byte & 0x7f) << (7 * (n % 4));
        }
        records.push(FileRecord {
            words,
            name,
            id: recipe.id,
        });
    }
    let mut on = records.iter().filter(|record| record.on()).count();
    for record in &mut kept {
        if record.on() {
            if on < ON_MAX {
                on += 1;
            } else {
                record.words[MODULE_WORD] |= OFF;
            }
        }
    }
    records.extend(kept);
    let (on, off): (Vec<_>, Vec<_>) = records.iter().partition(|record| record.on());
    let mut text = String::new();
    let mut put = |key: u32, value: u32| {
        if value != 0 {
            text.push_str(&format!("{key}\r\n{value}\r\n"));
        }
    };
    let names = RECIPE_WORDS * RECIPE_SLOTS;
    for (slot, record) in on.into_iter().chain(off).enumerate() {
        for (k, &value) in record.words.iter().enumerate() {
            put(recipe_key(slot * RECIPE_WORDS + k), value);
        }
        for (k, &value) in record.name.iter().enumerate() {
            put(recipe_key(names + slot * RECIPE_NAME_WORDS + k), value);
        }
        put(RECIPE_ID_KEY_BASE + slot as u32, record.id);
    }
    put(RECIPE_IMPORTED_KEY, value(RECIPE_IMPORTED_KEY));
    for u in 0..RECIPE_DIAL_POSITIONS as u32 {
        let assignment = value(RECIPE_DIAL_KEY_BASE + u);
        let held =
            assignment == RECIPE_DIAL_OFF || records.iter().any(|record| record.id == assignment);
        put(
            RECIPE_DIAL_KEY_BASE + u,
            if held { assignment } else { RECIPE_DIAL_FOLLOW },
        );
    }
    put(RECIPE_NEXT_ID_KEY, next);
    Ok(RecipeFile {
        text,
        recipes: written,
    })
}

/// The key and value pairs of a state file text.
fn file_values(text: &str) -> BTreeMap<u32, u32> {
    let mut values = BTreeMap::new();
    let mut lines = text.split_whitespace();
    while let (Some(key), Some(value)) = (lines.next(), lines.next()) {
        if let (Ok(key), Ok(value)) = (key.parse::<u32>(), value.parse::<u32>()) {
            values.insert(key, value);
        }
    }
    values
}
