//! The recipe library of a package, `RECIPES.BIN`.

use super::{Family, Recipe, LIBRARY_MAX, NAME_UNITS, ON_MAX};
use crate::abi::{
    RCP_BYTES, RCP_COUNT, RCP_CRC, RCP_DIAL, RCP_MAGIC, RCP_RECORD_FAMILY, RCP_RECORD_NAME,
    RCP_RECORD_WORDS, RECIPE_DIAL_POSITIONS,
};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};

/// The magic of the file.
pub const MAGIC: &[u8; 8] = b"GRKR0001";
/// The bytes before the first record.
pub const HEADER_SIZE: usize = 32;
/// The bytes of each record.
pub const RECORD_SIZE: usize = 64;

/// What INSTALL sets a user mode of the mode dial to when it imports a
/// library. The records of a library have no IDs, so a step names a record,
/// and INSTALL gives the user mode the ID of the recipe that the record
/// became on the camera. A record that INSTALL does not import, as one of
/// another camera line, makes the user mode follow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DialStep {
    /// Keep what the camera has, so a package that does not set the dial
    /// changes nothing.
    #[default]
    Keep,
    /// The user mode follows the dial.
    Follow,
    /// The user mode turns GRKit off.
    Off,
    /// The user mode applies the recipe of record k.
    Record(usize),
}

impl DialStep {
    /// The value of the step in the header (`GRKIT_RCP_DIAL_*`).
    pub fn code(self) -> u32 {
        match self {
            Self::Keep => 0,
            Self::Follow => 1,
            Self::Off => 2,
            Self::Record(k) => 3 + k as u32,
        }
    }

    fn from_code(code: u32, count: usize) -> Result<Self> {
        Ok(match code {
            0 => Self::Keep,
            1 => Self::Follow,
            2 => Self::Off,
            _ if ((code - 3) as usize) < count => Self::Record((code - 3) as usize),
            _ => bail!("The recipe library has a wrong user mode"),
        })
    }
}

/// What a library sets each user mode of the mode dial to, U1 to U3.
pub type LibraryDial = [DialStep; RECIPE_DIAL_POSITIONS];

/// A recipe library: the recipes, and what INSTALL sets the user modes of
/// the mode dial to.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    pub recipes: Vec<Recipe>,
    #[serde(default)]
    pub dial: LibraryDial,
}

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

/// The bytes of `RECIPES.BIN` for a library of `recipes` that sets the user
/// modes of the mode dial as `dial` gives. A user mode whose record is off
/// follows, as on the camera; a record that the library does not have is an
/// error.
pub fn encode(recipes: &[Recipe], dial: &LibraryDial) -> Result<Vec<u8>> {
    check_library(recipes)?;
    let mut bytes = vec![0; HEADER_SIZE + RECORD_SIZE * recipes.len()];
    bytes[RCP_MAGIC..RCP_MAGIC + 8].copy_from_slice(MAGIC);
    let size = bytes.len() as u32;
    put(&mut bytes, RCP_BYTES, size);
    put(&mut bytes, RCP_COUNT, recipes.len() as u32);
    for (u, step) in dial.iter().enumerate() {
        let step = match *step {
            DialStep::Record(k) => {
                let recipe = recipes.get(k).with_context(|| {
                    format!(
                        "U{} names recipe {}, which the library does not have",
                        u + 1,
                        k + 1
                    )
                })?;
                if recipe.enabled {
                    DialStep::Record(k)
                } else {
                    DialStep::Follow
                }
            }
            step => step,
        };
        put(&mut bytes, RCP_DIAL + 4 * u, step.code());
    }
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

/// The library of `RECIPES.BIN`. A damaged file is an error.
pub fn decode(bytes: &[u8]) -> Result<Library> {
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
    let mut dial = LibraryDial::default();
    for (u, step) in dial.iter_mut().enumerate() {
        *step = DialStep::from_code(get(bytes, RCP_DIAL + 4 * u), count)?;
    }
    let recipes = (0..count)
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
        .collect::<Result<_>>()?;
    Ok(Library { recipes, dial })
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
