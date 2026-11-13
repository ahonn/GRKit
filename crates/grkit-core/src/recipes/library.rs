//! The recipe library of a package, `RECIPES.BIN`.

use super::{Family, Recipe, LIBRARY_MAX, NAME_UNITS, ON_MAX};
use crate::abi::{
    RCP_BYTES, RCP_COUNT, RCP_CRC, RCP_MAGIC, RCP_RECORD_FAMILY, RCP_RECORD_NAME, RCP_RECORD_WORDS,
};
use anyhow::{ensure, Context, Result};

/// The magic of the file.
pub const MAGIC: &[u8; 8] = b"GRKR0001";
/// The bytes before the first record.
pub const HEADER_SIZE: usize = 32;
/// The bytes of each record.
pub const RECORD_SIZE: usize = 64;

/// Checks the size of a library: at most `LIBRARY_MAX` recipes, and at most
/// `ON_MAX` of them on.
pub fn check_library(recipes: &[Recipe]) -> Result<()> {
    ensure!(
        recipes.len() <= LIBRARY_MAX,
        "A library has at most {LIBRARY_MAX} recipes"
    );
    let on = recipes.iter().filter(|recipe| recipe.enabled).count();
    ensure!(on <= ON_MAX, "At most {ON_MAX} recipes are on, not {on}");
    Ok(())
}

/// The bytes of `RECIPES.BIN` for a library.
pub fn encode(recipes: &[Recipe]) -> Result<Vec<u8>> {
    check_library(recipes)?;
    let mut bytes = vec![0; HEADER_SIZE + RECORD_SIZE * recipes.len()];
    bytes[RCP_MAGIC..RCP_MAGIC + 8].copy_from_slice(MAGIC);
    let size = bytes.len() as u32;
    put(&mut bytes, RCP_BYTES, size);
    put(&mut bytes, RCP_COUNT, recipes.len() as u32);
    for (index, recipe) in recipes.iter().enumerate() {
        // An empty name is the automatic name of the camera, as in the recipe
        // file, so that INSTALL finds the recipe that the camera holds.
        let words = recipe
            .words()
            .with_context(|| format!("Recipe {}: {}", index + 1, recipe.name))?;
        let record = HEADER_SIZE + RECORD_SIZE * index;
        for (n, unit) in recipe.name.encode_utf16().enumerate() {
            bytes[record + RCP_RECORD_NAME + 2 * n..][..2].copy_from_slice(&unit.to_le_bytes());
        }
        for (n, word) in words.iter().enumerate() {
            put(&mut bytes, record + RCP_RECORD_WORDS + 4 * n, *word);
        }
        bytes[record + RCP_RECORD_FAMILY] = recipe.family.code();
    }
    let crc = checksum(&bytes);
    put(&mut bytes, RCP_CRC, crc);
    Ok(bytes)
}

/// The recipes of `RECIPES.BIN`. A damaged file is an error.
pub fn decode(bytes: &[u8]) -> Result<Vec<Recipe>> {
    ensure!(
        bytes.len() >= HEADER_SIZE && &bytes[RCP_MAGIC..RCP_MAGIC + 8] == MAGIC,
        "Not a recipe library"
    );
    let count = get(bytes, RCP_COUNT) as usize;
    ensure!(
        count <= LIBRARY_MAX
            && get(bytes, RCP_BYTES) as usize == bytes.len()
            && bytes.len() == HEADER_SIZE + RECORD_SIZE * count,
        "The recipe library has a wrong size"
    );
    ensure!(
        get(bytes, RCP_CRC) == checksum(bytes),
        "The recipe library is damaged"
    );
    (0..count)
        .map(|index| {
            let record = &bytes[HEADER_SIZE + RECORD_SIZE * index..][..RECORD_SIZE];
            let units: Vec<u16> = record[RCP_RECORD_NAME..RCP_RECORD_NAME + 2 * NAME_UNITS]
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .take_while(|&unit| unit != 0)
                .collect();
            let name = String::from_utf16(&units).context("A recipe name is not UTF-16")?;
            let words = std::array::from_fn(|n| get(record, RCP_RECORD_WORDS + 4 * n));
            let family = Family::from_code(record[RCP_RECORD_FAMILY])?;
            Recipe::from_words(name, family, words).with_context(|| format!("Recipe {}", index + 1))
        })
        .collect()
}

/// The CRC-32 of the file, which reads the CRC field as zero.
pub fn checksum(bytes: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&bytes[..RCP_CRC]);
    hasher.update(&[0; 4]);
    hasher.update(&bytes[RCP_CRC + 4..]);
    hasher.finalize()
}

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn get(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
