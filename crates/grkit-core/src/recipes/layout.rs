//! The bit layout of the `RECIPE_WORDS` words of a recipe. This is the one
//! source of the layout: the codec of this crate uses these values, and
//! `abi::header` writes each one to runtime/abi.h as `GRKIT_RECIPE_<NAME>`
//! for the core (runtime/recipes.h, runtime/native_look.h) and INSTALL
//! (runtime/install_recipes.h).
//!
//! A field is a `_SHIFT` and a `_MASK` (the mask of the field after the
//! shift). A flag is the value of its bit in the word. Each word stays below
//! 2^`VALUE_BITS`, as the state file keeps it.

use super::SETTINGS;

/// Declares each constant and `C_DEFINES`, the list that the C header gets.
macro_rules! layout {
    ($($(#[doc = $doc:literal])* $name:ident: $type:ty = $value:expr;)*) => {
        $($(#[doc = $doc])* pub const $name: $type = $value;)*
        /// Each constant of the layout by name, in the order of this file.
        pub const C_DEFINES: &[(&str, u32)] = &[$((stringify!($name), $name as u32)),*];
    };
}

layout! {
    /// The word of the Fn adjustments of an effect, or of the levels of a
    /// native type: base-9 digits, the first lowest.
    FN_WORD: usize = 1;
    /// The word of the native type and the white balance.
    NATIVE_WORD: usize = 2;
    /// The word of the color temperature, the corrections, and the native
    /// grain and Solarization of the type.
    EXTRA_WORD: usize = 3;
    /// The word of the first settings of modules, the exposure settings,
    /// and the off bit.
    MODULE_WORD: usize = 4;
    /// The word of the settings of modules after those of MODULE_WORD.
    SETTINGS_WORD: usize = 5;
    /// The bits of each word: a word is below 2^VALUE_BITS.
    VALUE_BITS: u32 = 29;

    // Word 0.
    /// The look: kept, Film Look Off, or a GRKit effect.
    LOOK_MASK: u32 = 3;
    LOOK_KEPT: u32 = 0;
    LOOK_OFF: u32 = 1;
    LOOK_EFFECT: u32 = 2;
    /// The recipe holds Grain: its level and its size.
    GRAIN_HELD: u32 = 4;
    GRAIN_LEVEL_SHIFT: u32 = 3;
    GRAIN_LEVEL_MASK: u32 = 7;
    GRAIN_SIZE_SHIFT: u32 = 6;
    GRAIN_SIZE_MASK: u32 = 3;
    /// The low 15 bits of the ImageTone code of the effect.
    CODE_SHIFT: u32 = 8;
    CODE_MASK: u32 = 0x7fff;
    /// The number after the automatic name, 0 for none.
    NUMBER_SHIFT: u32 = 23;
    NUMBER_MAX: u32 = 15;
    /// Set in each recipe, so word 0 of a recipe is never 0.
    USED: u32 = 1 << 27;
    /// The recipe stays out of the rows of ADJ.
    HIDDEN: u32 = 1 << 28;

    // Word 2: the type + 1 (0: no type), its tone, the HDR intensity, the
    // B&W filter, the white balance mode + 1 (0: no mode), and its A-B and
    // G-M shifts.
    TYPE_SHIFT: u32 = 0;
    TYPE_MASK: u32 = 31;
    TONE_SHIFT: u32 = 5;
    TONE_MASK: u32 = 15;
    INTENSITY_SHIFT: u32 = 9;
    INTENSITY_MASK: u32 = 3;
    FILTER_SHIFT: u32 = 11;
    FILTER_MASK: u32 = 7;
    MODE_SHIFT: u32 = 14;
    MODE_MASK: u32 = 31;
    BA_SHIFT: u32 = 19;
    GM_SHIFT: u32 = 24;
    /// The mask of each white balance shift.
    SHIFT_MASK: u32 = 31;
    /// The bits of the type, and those of the white balance.
    LOOK_BITS: u32 = (1 << MODE_SHIFT) - 1;
    MODE_BITS: u32 = ((1 << VALUE_BITS) - 1) & !LOOK_BITS;

    // Word 3: the color temperature, (kelvin - 2500) / 10; each correction
    // setting, its stored value + 1 (0: Unset); the native grain of the
    // type; and its Solarization type.
    TEMPERATURE_MASK: u32 = 1023;
    CORRECTION_SHIFT: u32 = 10;
    /// The bits of each correction setting, from CORRECTION_SHIFT.
    CORRECTION_BITS: u32 = 3;
    CORRECTION_MASK: u32 = 7;
    /// The native grain: the strength, 0 Off and 1 to 3 Low to High, and
    /// the size, 0 to 2 Small to Large.
    NATIVE_GRAIN_SHIFT: u32 = 22;
    NATIVE_GRAIN_MASK: u32 = 3;
    NATIVE_GRAIN_SIZE_SHIFT: u32 = 24;
    NATIVE_GRAIN_SIZE_MASK: u32 = 3;
    NATIVE_GRAIN_BITS: u32 = 15 << NATIVE_GRAIN_SHIFT;
    SOLARIZATION_SHIFT: u32 = 26;
    SOLARIZATION_MASK: u32 = 7;
    /// The bits of word 3 that belong to the type.
    TYPE_EXTRA_BITS: u32 = 0x7f << NATIVE_GRAIN_SHIFT;

    // Word 4: the first settings of modules (SETTING_PLACES), then the
    // exposure settings, each 0 when the recipe does not hold it.
    /// The compensation in sixths of a stop + EV_ZERO.
    EV_SHIFT: u32 = 9;
    EV_MASK: u32 = 63;
    EV_ZERO: u32 = 31;
    /// The third stop of the Auto ISO upper limit from ISO 100, + 1.
    ISO_SHIFT: u32 = 15;
    ISO_MASK: u32 = 63;
    /// The stored metering + 1.
    METERING_SHIFT: u32 = 21;
    METERING_MASK: u32 = 7;
    /// The stored ND filter + 1.
    ND_SHIFT: u32 = 24;
    ND_MASK: u32 = 3;
    /// The bits of the exposure settings.
    EXPOSURE_BITS: u32 = ((1 << (ND_SHIFT + 2)) - 1) & !((1 << EV_SHIFT) - 1);
    /// The recipe is off.
    OFF: u32 = 1 << 28;
}

/// The place of each setting of `SETTINGS`: its word and the shift of its
/// bits there. Bit 0 holds the setting, and the bits of its value follow.
/// The settings fill MODULE_WORD below the exposure settings, then
/// SETTINGS_WORD, in order, so a new setting moves no other.
pub const SETTING_PLACES: [(usize, u32); SETTINGS.len()] = {
    let mut places = [(0, 0); SETTINGS.len()];
    let (mut word, mut shift, mut i) = (MODULE_WORD, 0, 0);
    while i < SETTINGS.len() {
        let width = 1 + SETTINGS[i].bits;
        if word == MODULE_WORD && shift + width > EV_SHIFT {
            (word, shift) = (SETTINGS_WORD, 0);
        }
        assert!(
            shift + width <= VALUE_BITS,
            "the settings fit in SETTINGS_WORD"
        );
        places[i] = (word, shift);
        shift += width;
        i += 1;
    }
    places
};

/// The mask of the value of each setting, after the shift of its place and
/// the bit that holds it.
pub const SETTING_MASKS: [u32; SETTINGS.len()] = {
    let mut masks = [0; SETTINGS.len()];
    let mut i = 0;
    while i < SETTINGS.len() {
        masks[i] = (1 << SETTINGS[i].bits) - 1;
        assert!(
            SETTINGS[i].values as u32 <= masks[i] + 1,
            "each value fits in its bits"
        );
        assert!(
            !SETTINGS[i].with_previous || i > 0,
            "a setting follows the one that it needs"
        );
        i += 1;
    }
    masks
};

const _: () = {
    assert!(CORRECTION_SHIFT + CORRECTION_BITS * 4 == NATIVE_GRAIN_SHIFT);
    assert!(ND_MASK == 3 && OFF >> VALUE_BITS == 0 && HIDDEN >> VALUE_BITS == 0);
    assert!((NUMBER_MAX << NUMBER_SHIFT) & USED == 0);
};
