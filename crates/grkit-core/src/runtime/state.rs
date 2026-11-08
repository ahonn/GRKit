//! The state files of the resident loader: the key IDs, the GRKit 1.2 keys,
//! and the largest file.

use super::{
    MAX_COUNT, MODULE_SETTINGS_MAX, MODULE_SETTING_VALUES, RECIPE_DIAL_POSITIONS,
    RECIPE_NAME_WORDS, RECIPE_SLOTS, RECIPE_WORDS,
};
use crate::capabilities::{self, StateFile};

/// The largest main state file that INSTALL reads (`STATE_TEXT_MAX` in
/// runtime/installer.c). It leaves room for keys of a later release.
pub const STATE_MAX_BYTES: usize = 16384;

/// A state file keeps the state keys of the capabilities
/// (`capabilities::state_keys`) and the saved module settings.
/// The file holds two decimal lines per saved key: its ID (`state_key_id`),
/// then its value. Module settings include zero, so a missing key can use
/// its declared default. Core settings with value zero can omit their key.
/// The most digits of a number in the state file. The loader parses the digits
/// itself, and 9 digits cannot overflow its 32-bit integers.
pub const STATE_DIGITS: usize = 9;

/// The ID of a state key in the file, as for a module setting
/// (`grm::state_id`): the CRC-32 of its name, modulo 10^9, so it has at most
/// `STATE_DIGITS` digits, out of the range of the Fn adjustments and not 0.
pub fn state_key_id(name: &str) -> u32 {
    let mut id = crc32fast::hash(name.as_bytes()) % 1_000_000_000;
    if id >= PARAMETER_KEY_BASE {
        id -= 10_000_000;
    }
    id.max(1)
}

/// The Fn adjustments of an effect are a state entry of their own: the key ID
/// is this base plus the low 15 bits of the effect's ImageTone code, and the
/// value is the nine stored bytes (`RECORD_PARAMETERS`) as base-9 digits, the
/// first adjustment lowest. Neutral adjustments have no entry.
pub const PARAMETER_KEY_BASE: u32 = 990_000_000;
/// The recipes (`runtime/recipes.h`) have the recipe file
/// (`StateFile::Recipes`). Each word of a recipe has its own range of keys:
/// word w of recipe n has the key ID this base + w * `RECIPE_KEY_STRIDE` + n.
/// Word w of the name of recipe n has `RECIPE_NAME_KEY_BASE` + w *
/// `RECIPE_KEY_STRIDE` + n. A later word of a recipe or a name gets the next
/// range, so the keys of a saved file stay valid. The words after the names
/// have one key each from `RECIPE_IMPORTED_KEY`, in the order of the header:
/// the hash of the imported package library, the ID of each recipe, the
/// assignment of each user mode of the mode dial, and the next ID. The header
/// holds the words recipe by recipe (`HOST_RECIPES`, `HOST_RECIPE_NAMES`,
/// then `HOST_RECIPE_IMPORTED` to `HOST_RECIPE_NEXT_ID`). A word that is 0
/// has no entry. The undo steps are not saved.
pub const RECIPE_KEY_BASE: u32 = 991_000_000;
pub const RECIPE_KEY_STRIDE: u32 = 100;
pub const RECIPE_NAME_KEY_BASE: u32 = RECIPE_KEY_BASE + 50 * RECIPE_KEY_STRIDE;
pub const RECIPE_IMPORTED_KEY: u32 = RECIPE_KEY_BASE + 90 * RECIPE_KEY_STRIDE;
pub const RECIPE_ID_KEY_BASE: u32 = RECIPE_IMPORTED_KEY + 1;
pub const RECIPE_DIAL_KEY_BASE: u32 = RECIPE_ID_KEY_BASE + RECIPE_SLOTS as u32;
pub const RECIPE_NEXT_ID_KEY: u32 = RECIPE_DIAL_KEY_BASE + RECIPE_DIAL_POSITIONS as u32;
const _: () = assert!(RECIPE_NEXT_ID_KEY < RECIPE_IMPORTED_KEY + RECIPE_KEY_STRIDE);
const _: () = assert!(RECIPE_SLOTS <= RECIPE_KEY_STRIDE as usize);
const _: () = assert!(RECIPE_WORDS <= 50 && RECIPE_NAME_WORDS <= 40);

/// The state key of header word `index` of the recipe run (`HOST_RECIPES`,
/// the names, then the words from the import hash to the next ID).
pub fn recipe_key(index: usize) -> u32 {
    let words = RECIPE_WORDS * RECIPE_SLOTS;
    if index < words {
        RECIPE_KEY_BASE
            + (index % RECIPE_WORDS) as u32 * RECIPE_KEY_STRIDE
            + (index / RECIPE_WORDS) as u32
    } else if index < words + RECIPE_NAME_WORDS * RECIPE_SLOTS {
        let name = index - words;
        RECIPE_NAME_KEY_BASE
            + (name % RECIPE_NAME_WORDS) as u32 * RECIPE_KEY_STRIDE
            + (name / RECIPE_NAME_WORDS) as u32
    } else {
        RECIPE_IMPORTED_KEY + (index - words - RECIPE_NAME_WORDS * RECIPE_SLOTS) as u32
    }
}
/// The largest recipe key.
pub fn recipe_key_max() -> u32 {
    (0..RECIPE_STATE_WORDS).map(recipe_key).max().unwrap()
}
/// The number of saved recipe words: the recipes, their names, the import
/// hash, the IDs, the user modes of the mode dial, and the next ID.
pub const RECIPE_STATE_WORDS: usize = RECIPE_WORDS * RECIPE_SLOTS
    + RECIPE_NAME_WORDS * RECIPE_SLOTS
    + 1
    + RECIPE_SLOTS
    + RECIPE_DIAL_POSITIONS
    + 1;
/// The largest value of a recipe word. Each word holds at most 29 bits, and
/// a name word 28 bits.
pub const RECIPE_VALUE_MAX: u32 = (1 << 29) - 1;
/// The IDs of recipes: 1 to `RECIPE_ID_MAX` (`abi::HOST_RECIPE_IDS`).
pub const RECIPE_ID_MAX: u32 = RECIPE_VALUE_MAX - 1;
/// What a user mode of the mode dial does (`abi::HOST_RECIPE_DIAL`): the
/// camera stays as it is (0), GRKit is off, or the ID of a recipe applies.
pub const RECIPE_DIAL_FOLLOW: u32 = 0;
pub const RECIPE_DIAL_OFF: u32 = RECIPE_VALUE_MAX;
/// The largest packed adjustment value: 9^9 - 1.
pub const PARAMETER_VALUE_MAX: u32 = 387_420_488;
/// The number of adjustments of an effect.
pub const PARAMETER_COUNT: usize = 9;
/// The Toning of a native look (`abi::RECORD_TONING`) is an entry of its own: the
/// key ID is this base plus the low 15 bits of the effect's ImageTone code, and the
/// value is the stored byte, 1..=8. The look's own toning has no entry.
pub const TONING_KEY_BASE: u32 = PARAMETER_KEY_BASE + 32768;
const _: () = assert!(TONING_KEY_BASE + 32767 < 1_000_000_000);

/// The state keys of GRKit 1.2 and the keys that now hold the same values.
/// The values keep their meaning, and the Fn adjustment entries keep their
/// key IDs.
pub const LEGACY_STATE_KEYS: [(&str, &str); 6] = [
    ("image-control.effect", "CORE.TONE"),
    ("framing.ratio", "CORE.ASPECT"),
    ("guides.style", "GUIDES.STYLE"),
    ("date-stamp.style", "DATE.STYLE"),
    ("date-stamp.look", "DATE.LOOK"),
    ("grain.level", "CORE.GRAIN"),
];

/// The ID of a GRKit 1.2 state key in its state file: the CRC-32 of its
/// name, modulo 10^9.
pub fn legacy_state_key_id(name: &str) -> u32 {
    crc32fast::hash(name.as_bytes()) % 1_000_000_000
}

/// The pairs of (GRKit 1.2 key ID, current key ID) of `LEGACY_STATE_KEYS`.
pub fn legacy_state_ids() -> Vec<(u32, u32)> {
    LEGACY_STATE_KEYS
        .iter()
        .map(|(old, new)| (legacy_state_key_id(old), state_key_id(new)))
        .collect()
}

/// The size of the largest file that the loader writes for `file`: each key
/// at its largest value, and for the main file, an adjustment entry for each
/// effect. Each line ends with CR LF.
pub fn worst_state_size(file: StateFile) -> usize {
    let line = |value: u32| value.to_string().len() + 2;
    let keys: usize = capabilities::state_keys()
        .iter()
        .filter(|key| key.file == file)
        .map(|key| line(state_key_id(&key.name)) + line(key.max))
        .sum();
    let parameters = match file {
        StateFile::Main => {
            MAX_COUNT * (line(PARAMETER_KEY_BASE + 32767) + line(PARAMETER_VALUE_MAX))
                + MAX_COUNT * (line(TONING_KEY_BASE + 32767) + line(8))
                + MODULE_SETTINGS_MAX
                    * (line(PARAMETER_KEY_BASE - 1) + line(MODULE_SETTING_VALUES as u32 - 1))
        }
        StateFile::Recipes => {
            RECIPE_STATE_WORDS * (line(recipe_key_max()) + line(RECIPE_VALUE_MAX))
        }
    };
    keys + parameters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_keys_have_distinct_positive_ids() {
        let keys = capabilities::state_keys();
        let mut ids: Vec<_> = keys.iter().map(|key| state_key_id(&key.name)).collect();
        assert!(ids
            .iter()
            .all(|id| *id > 0 && id.to_string().len() <= STATE_DIGITS));
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), keys.len());
        // The loader handles the effect of CORE.TONE, the one key without a
        // header word, as value 0.
        assert_eq!(keys[0].name, "CORE.TONE");
        assert!(keys[0].word.is_none());
        assert!(keys[1..].iter().all(|key| key.word.is_some()));
        // ADJ open preference and last GRKit root module persist across
        // power-off; they have no firmware points and INSTALL always
        // restores them.
        assert_eq!(keys[5].name, "CORE.ADJ");
        assert_eq!(keys[5].word, Some(crate::abi::HOST_ADJ_OPEN));
        assert_eq!(keys[6].name, "CORE.ADJ_MODULE");
        assert_eq!(keys[6].word, Some(crate::abi::HOST_MODULE));
        assert_eq!(state_key_id("CORE.ADJ"), 458876999);
        assert_eq!(state_key_id("CORE.ADJ_MODULE"), 697053196);
    }

    #[test]
    fn legacy_state_keys_map_to_the_current_keys() {
        let ids = legacy_state_ids();
        let mut old: Vec<_> = ids.iter().map(|(old, _)| *old).collect();
        old.sort_unstable();
        old.dedup();
        assert_eq!(old.len(), LEGACY_STATE_KEYS.len());
        // An old key is neither an Fn adjustment entry nor 0.
        assert!(old
            .iter()
            .all(|id| *id > 0 && !(PARAMETER_KEY_BASE..=PARAMETER_KEY_BASE + 32767).contains(id)));
        // The key IDs in the state files of GRKit 1.2.
        assert_eq!(
            old,
            [161811731, 495043802, 541741532, 684634731, 733922165, 957718554]
        );
        // Each target is a capability or a setting of an official module.
        let capabilities: Vec<_> = capabilities::state_keys()
            .iter()
            .map(|key| state_key_id(&key.name))
            .collect();
        let modules = [
            crate::grm::state_id("GUIDES", "STYLE"),
            crate::grm::state_id("DATE", "STYLE"),
            crate::grm::state_id("DATE", "LOOK"),
        ];
        for (_, new) in &ids {
            assert!(capabilities.contains(new) || modules.contains(new));
        }
    }

    #[test]
    fn adjustment_entries_fit_the_state_file() {
        // Nine base-9 digits, and keys of at most nine digits that no other
        // key uses.
        assert_eq!(PARAMETER_VALUE_MAX, 9u32.pow(PARAMETER_COUNT as u32) - 1);
        assert_eq!(
            PARAMETER_COUNT,
            crate::abi::RECORD_FIELDS
                .iter()
                .find(|field| field.name == "RECORD_PARAMETERS")
                .unwrap()
                .size
        );
        let last = PARAMETER_KEY_BASE + 32767;
        assert!(last.to_string().len() <= STATE_DIGITS);
        assert!(capabilities::state_keys().iter().all(|key| {
            let id = state_key_id(&key.name);
            !(PARAMETER_KEY_BASE..=last).contains(&id)
        }));
        // Each key and value line, with CR LF, for every effect, capability key,
        // and module setting.
        let line = STATE_DIGITS + 2;
        assert!(
            worst_state_size(StateFile::Main)
                <= 2 * line * (MAX_COUNT + capabilities::state_keys().len() + MODULE_SETTINGS_MAX)
        );
        // Module setting IDs stay below the adjustment entries, and values
        // fit the digits that the loader parses.
        assert!(((MODULE_SETTING_VALUES - 1).to_string().len()) <= STATE_DIGITS);
        assert!(crate::grm::state_id("GUIDES", "STYLE") < PARAMETER_KEY_BASE);
    }

    /// INSTALL reads the largest file that the loader writes: its bytes and
    /// its pairs (runtime/state.h).
    #[test]
    fn the_read_limit_covers_the_largest_state_file() {
        for file in StateFile::ALL {
            assert!(worst_state_size(file) <= STATE_MAX_BYTES, "{file:?}");
        }
        let header = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../runtime/state.h"),
        )
        .unwrap();
        let pairs: usize = header
            .lines()
            .find_map(|line| line.strip_prefix("#define STATE_PAIRS "))
            .and_then(|value| value.trim_end_matches('U').parse().ok())
            .unwrap();
        assert!(MAX_COUNT + capabilities::state_keys().len() + MODULE_SETTINGS_MAX <= pairs);
        assert!(RECIPE_STATE_WORDS <= pairs);
    }

    /// The recipe entries keep apart from the Fn adjustment entries and the
    /// IDs of names, and fit the digits of the state file. Each word has its
    /// own key.
    #[test]
    fn recipe_entries_fit_the_state_file() {
        let keys: std::collections::BTreeSet<u32> =
            (0..RECIPE_STATE_WORDS).map(recipe_key).collect();
        assert_eq!(keys.len(), RECIPE_STATE_WORDS);
        let last = recipe_key_max();
        const { assert!(RECIPE_KEY_BASE > PARAMETER_KEY_BASE + 32767) };
        assert!(last.to_string().len() <= STATE_DIGITS);
        assert!(RECIPE_VALUE_MAX.to_string().len() <= STATE_DIGITS);
        assert!(capabilities::state_keys().iter().all(|key| {
            let id = state_key_id(&key.name);
            !(RECIPE_KEY_BASE..=last).contains(&id)
        }));
        // Word 1 of recipe 2, word 0 of recipe 39, word 3 of the name of
        // recipe 7, and the import hash.
        assert_eq!(recipe_key(2 * RECIPE_WORDS + 1), 991_000_102);
        assert_eq!(recipe_key(39 * RECIPE_WORDS), 991_000_039);
        assert_eq!(
            recipe_key(RECIPE_WORDS * RECIPE_SLOTS + 7 * RECIPE_NAME_WORDS + 3),
            991_005_307
        );
        let tail = RECIPE_WORDS * RECIPE_SLOTS + RECIPE_NAME_WORDS * RECIPE_SLOTS;
        assert_eq!(recipe_key(tail), RECIPE_IMPORTED_KEY);
        assert_eq!(recipe_key(tail + 1 + 5), RECIPE_ID_KEY_BASE + 5);
        assert_eq!(
            recipe_key(tail + 1 + RECIPE_SLOTS + 2),
            RECIPE_DIAL_KEY_BASE + 2
        );
        assert_eq!(recipe_key(RECIPE_STATE_WORDS - 1), RECIPE_NEXT_ID_KEY);
    }
}
