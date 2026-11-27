//! The control mailbox between the Wi-Fi service on Linux and the App task
//! (runtime/control_mailbox.h, runtime/control.h): its layout, its result
//! codes, and the commands with the layout of their payloads.

use super::*;

header_consts!(CONTROL_CONSTS {
/// The mailbox from the Wi-Fi service (Linux) to the App task
/// (runtime/control_mailbox.h). Offsets are bytes. The runtime owns the
/// descriptor and the result fields; the service owns the request and the
/// release. The service installs a nonzero epoch with `CONTROL_ATTACH`, once
/// for each start of the camera.
pub const CONTROL_MAGIC_VALUE: u32 = 0x47524331;
pub const CONTROL_VERSION_VALUE: u32 = 1;
pub const CONTROL_PAYLOAD_BYTES: usize = 2048;
pub const CONTROL_SEQUENCE_MAX: u32 = 0x7fff_ffff;
pub const CONTROL_EMPTY: u32 = 0;
pub const CONTROL_ACCEPTED: u32 = 1;
pub const CONTROL_BAD_CONTEXT: u32 = 16;
pub const CONTROL_BAD_LENGTH: u32 = 17;
pub const CONTROL_BAD_DIGEST: u32 = 18;
pub const CONTROL_CHANGED: u32 = 19;
pub const CONTROL_UNSUPPORTED: u32 = 20;
pub const CONTROL_INVALID: u32 = 21;
pub const CONTROL_BUSY: u32 = 22;
/// The change did not apply: ADJ showed, and the App task closes it for the
/// change. The service sends the same command again.
pub const CONTROL_CLOSING: u32 = 23;
pub const CONTROL_ATTACH: u32 = 1;
pub const CONTROL_PING: u32 = 2;
/// The listed effects (runtime/control_looks.h). No payload. The response is
/// `CONTROL_LOOKS_HEADER` bytes: the number of effects, the key of the
/// committed effect or 0, the count of state changes (`HOST_STATE_CHANGES`),
/// and the number of codes in use. Then `CONTROL_LOOK_ENTRY` bytes for each
/// effect: its key (the state ID of `CORE.EFFECT.<ID>`), its choice (the
/// position in bits 1 and up, bit 0 set for Off), and 1 if it has a record.
/// Then one word for each recipe or mode-dial set that names an effect: the
/// recipe code of that effect in the low 15 bits, where it is from
/// (`CONTROL_USE_ORIGIN_SHIFT`: the recipe slot + 1, or with
/// `CONTROL_USE_DIAL` the user mode of the dial), the number of the recipe
/// (`CONTROL_USE_NUMBER_SHIFT`), and with `CONTROL_USE_NAMED` the
/// `RECIPE_NAME_WORDS` name words of the recipe after it, while the payload
/// has room for them.
pub const CONTROL_LOOKS_READ: u32 = 3;
/// The recipe code of a use word: its low 15 bits.
pub const CONTROL_USE_CODE_MASK: u32 = 0x7fff;
pub const CONTROL_USE_ORIGIN_SHIFT: u32 = 16;
pub const CONTROL_USE_ORIGIN_MASK: u32 = 0x7f;
pub const CONTROL_USE_DIAL: u32 = 1 << 23;
pub const CONTROL_USE_NUMBER_SHIFT: u32 = 24;
pub const CONTROL_USE_NUMBER_MASK: u32 = 0xf;
pub const CONTROL_USE_NAMED: u32 = 1 << 31;
/// Turn an effect on or off, as its checkbox in MENU does. The payload is
/// the key and 1 for Off or 0 for On.
pub const CONTROL_LOOK_TURN: u32 = 4;
/// Set the order of the effects, as Order in MENU does. The payload is the
/// key of each listed effect, in the new order.
pub const CONTROL_LOOKS_ORDER: u32 = 5;
pub const CONTROL_LOOKS_HEADER: usize = 16;
/// The recipes (runtime/control_recipes.h), `CONTROL_RECIPES_PAGE` at a
/// time. The payload is the first slot. The response is
/// `CONTROL_RECIPES_HEADER` bytes: the number of recipes, the number that
/// are on (they come first), the slot of the first recipe that is on and
/// matches the current settings or `CONTROL_RECIPE_NONE`, the count of
/// recipe changes (`HOST_RECIPE_CHANGES`), the number of undo steps, the
/// first slot, the number of entries, and the native Image Control type of
/// the camera plus 1, numbered as a recipe numbers it (0 before the camera
/// has its settings). Then
/// `CONTROL_RECIPE_ENTRY` bytes
/// for each recipe: its ID, its `RECIPE_WORDS` words, and its
/// `RECIPE_NAME_WORDS` name words.
pub const CONTROL_RECIPES_READ: u32 = 6;
/// Turn a recipe on or off: its ID, and 1 for Off or 0 for On. A recipe that
/// turns on goes after the others that are on, one that turns off before
/// the others that are off.
pub const CONTROL_RECIPE_TURN: u32 = 7;
/// The order of the recipes that are on: the ID of each, in the new order.
pub const CONTROL_RECIPES_ORDER: u32 = 8;
/// Rename a recipe: its ID and at most `RECIPE_NAME_CHARS` ASCII bytes; no
/// bytes give the automatic name.
pub const CONTROL_RECIPE_RENAME: u32 = 9;
/// Delete a recipe: its ID.
pub const CONTROL_RECIPE_DELETE: u32 = 10;
/// Apply a recipe, as OK in ADJ does: its ID. The response is the result
/// of the apply (RECIPE_READY with the skipped bits, or the reason).
pub const CONTROL_RECIPE_APPLY: u32 = 11;
/// Undo the newest apply. The response is the number of undo steps left.
pub const CONTROL_RECIPE_UNDO: u32 = 12;
/// Add a recipe: its `RECIPE_WORDS` words and `RECIPE_NAME_WORDS` name
/// words. It is on after the others that are on while fewer than
/// `RECIPE_ON_MAX` are on, else off. The response is its ID.
pub const CONTROL_RECIPE_ADD: u32 = 13;
/// The state of the listed modules (runtime/control_modules.h), in the order
/// of ADJ Order: toolbar, root, hidden, then modules that ADJ cannot show.
/// The payload is the first row. The response is `CONTROL_MODULES_HEADER`
/// bytes: the number of modules, the count of state changes, 1 while GRKit
/// runs, the first row, the number of entries, and the capabilities that
/// run (`GRKIT_HOST_CAPABILITIES`). Then, for each module that the camera
/// can use:
/// its index with `CONTROL_MODULE_*` fields, its name in 8 ASCII bytes, the
/// mask of the values that are off in ADJ, the number of settings and in
/// bits 8 and up the number of values in its ADJ order, a key and a value
/// for each setting, then the ADJ order, a byte each, padded to a word.
pub const CONTROL_MODULES_READ: u32 = 14;
/// The texts of a module in one language, as many records as fit. The
/// payload is the module, the language (`Language::ALL`), and the cursor.
/// The response is the next cursor or `CONTROL_MODULE_TEXTS_END`, and the
/// number of records. A record is a word with its kind and in bits 8 and up
/// its number of UTF-16 units, two argument words, and the units padded to
/// a word. The module record comes first; a group record precedes the
/// settings of a child page; a value record follows its setting.
pub const CONTROL_MODULE_TEXTS: u32 = 15;
/// Bytes of tile 0 of a module: the module and a word-aligned offset below
/// `effects::ICON_SIZE`. The response is the next bytes of its RGBA pixels.
pub const CONTROL_MODULE_TILE: u32 = 16;
/// Change a module in one step, after the check of each field: the module,
/// the `CONTROL_MODULE_SET_*` fields, 1 for Off, the mask of hidden values,
/// the number of settings, the number of values in the ADJ order, a key and
/// a value for each setting, then each value of the ADJ order as a word.
pub const CONTROL_MODULE_SET: u32 = 17;
/// The new order of the modules of the ADJ root: the index of each, once.
pub const CONTROL_MODULES_ORDER: u32 = 18;
/// The GRKit settings of MENU > GRKit Settings (runtime/manage.h), no
/// payload. The response is a word for each `CONTROL_SETTING_*` in order:
/// `CONTROL_SETTING_NONE` when the camera does not have the setting, else
/// its value (Enable at Start: `MANAGE_START_*`; Distance Unit: 0 meters, 1
/// feet; the others: 0 Off, 1 On).
pub const CONTROL_SETTINGS_READ: u32 = 19;
/// Set one GRKit setting as its MENU row does: the `CONTROL_SETTING_*` and
/// the value, as `CONTROL_SETTINGS_READ` gives them.
pub const CONTROL_SETTINGS_SET: u32 = 20;
/// Place the modules of MENU > ADJ Order in one step: a word for each module
/// that ADJ can show, each once, with its index in the low byte and its
/// place at `CONTROL_MODULE_PLACE_SHIFT`: first the toolbar in its order (at
/// most `capabilities::ADJ_ENTRIES`), then the root in its order, then the
/// hidden modules.
pub const CONTROL_MODULES_LAYOUT: u32 = 21;
pub const CONTROL_SETTING_START: u32 = 0;
pub const CONTROL_SETTING_RAW: u32 = 1;
pub const CONTROL_SETTING_UNIT: u32 = 2;
pub const CONTROL_SETTING_BATTERY: u32 = 3;
pub const CONTROL_SETTING_BATTERY_PERCENT: u32 = 4;
pub const CONTROL_SETTING_PLAYBACK_INFO: u32 = 5;
pub const CONTROL_SETTING_GOODBYE: u32 = 6;
pub const CONTROL_SETTING_GOODBYE_TEXT: u32 = 7;
pub const CONTROL_SETTINGS: u32 = 8;
pub const CONTROL_SETTING_NONE: u32 = u32::MAX;
pub const CONTROL_MODULES_HEADER: usize = 24;
pub const CONTROL_MODULE_ENTRY: usize = 20;
/// The byte offsets of the fields of a `CONTROL_MODULES_READ` entry after its
/// first word: the name, the mask of hidden values, and the sizes.
pub const CONTROL_MODULE_NAME_AT: usize = 4;
pub const CONTROL_MODULE_NAME_BYTES: usize = 8;
pub const CONTROL_MODULE_HIDDEN_AT: usize = 12;
pub const CONTROL_MODULE_SIZES_AT: usize = 16;
pub const CONTROL_MODULE_TEXTS_HEADER: usize = 8;
pub const CONTROL_MODULE_TEXTS_END: u32 = u32::MAX;
/// The longest text of a record, in UTF-16 units.
pub const CONTROL_MODULE_TEXT_UNITS: u32 = 120;
pub const CONTROL_MODULE_RECORD_MODULE: u32 = 0;
pub const CONTROL_MODULE_RECORD_GROUP: u32 = 1;
pub const CONTROL_MODULE_RECORD_SETTING: u32 = 2;
pub const CONTROL_MODULE_RECORD_VALUE: u32 = 3;
/// The first argument of a module record: in its low byte
/// (`CONTROL_MODULE_MANAGED_MASK`) the page that the phone manages in a tab of
/// its own, or 0, and these bits. The second argument is the key of the page
/// setting.
pub const CONTROL_MODULE_MANAGED_MASK: u32 = 0xff;
pub const CONTROL_MODULE_MANAGED_LOOKS: u32 = 1;
pub const CONTROL_MODULE_MANAGED_RECIPES: u32 = 2;
pub const CONTROL_MODULE_HAS_OPTIONS: u32 = 1 << 8;
pub const CONTROL_MODULE_ORDERED: u32 = 1 << 9;
/// The key of a setting without a state ID: this bit and its handle. A
/// state ID is below 10^9, so the two ranges do not meet.
pub const CONTROL_MODULE_KEY_HANDLE: u32 = 0x4000_0000;
/// The first word of a module entry: the index in the low byte, then these.
pub const CONTROL_MODULE_STATUS_SHIFT: u32 = 8;
pub const CONTROL_MODULE_STATUS_MASK: u32 = 3;
pub const CONTROL_MODULE_RUNNING: u32 = 0;
pub const CONTROL_MODULE_STOPPED: u32 = 1;
pub const CONTROL_MODULE_SAFE: u32 = 2;
pub const CONTROL_MODULE_FAILED: u32 = 3;
pub const CONTROL_MODULE_PLACE_SHIFT: u32 = 10;
pub const CONTROL_MODULE_PLACE_MASK: u32 = 3;
pub const CONTROL_MODULE_TOOLBAR: u32 = 0;
pub const CONTROL_MODULE_ROOT: u32 = 1;
pub const CONTROL_MODULE_HIDDEN: u32 = 2;
pub const CONTROL_MODULE_ORDERABLE: u32 = 1 << 12;
pub const CONTROL_MODULE_ENABLED: u32 = 1 << 13;
/// ADJ can show the module, so `CONTROL_MODULES_LAYOUT` places it.
pub const CONTROL_MODULE_PLACEABLE: u32 = 1 << 14;
pub const CONTROL_MODULE_SET_ENABLED: u32 = 1;
pub const CONTROL_MODULE_SET_HIDDEN: u32 = 2;
pub const CONTROL_MODULE_SET_ORDER: u32 = 4;
pub const CONTROL_RECIPES_HEADER: usize = 32;
pub const CONTROL_RECIPE_ENTRY: usize = 4 * (1 + RECIPE_WORDS + RECIPE_NAME_WORDS);
pub const CONTROL_RECIPES_PAGE: usize = 32;
pub const CONTROL_RECIPE_NONE: u32 = u32::MAX;
pub const CONTROL_LOOK_ENTRY: usize = 12;
pub const CONTROL_REQUEST_SIZE: usize = 16 + CONTROL_PAYLOAD_BYTES;
});
const _: () = assert!(
    CONTROL_MODULE_NAME_AT + CONTROL_MODULE_NAME_BYTES == CONTROL_MODULE_HIDDEN_AT
        && CONTROL_MODULE_SIZES_AT + 4 == CONTROL_MODULE_ENTRY
);
packed_fields!(CONTROL_FIELDS, CONTROL_SIZE {
    CONTROL_MAGIC: 4;
    CONTROL_VERSION: 4;
    CONTROL_CAPACITY: 4;
    CONTROL_EPOCH: 16;
    CONTROL_STATE_CHANGES: 4;
    CONTROL_DESCRIPTOR_PADDING: 32;
    CONTROL_REQUEST: 4;
    CONTROL_RELEASE: 4;
    CONTROL_REQUEST_EPOCH: 16;
    CONTROL_COMMAND: 4;
    CONTROL_LENGTH: 4;
    CONTROL_CHECKSUM: 4;
    /// CONTROL_SERVICE_RUNNING while the Wi-Fi service runs: the service
    /// writes it as it starts and clears it as it stops. The resident loader
    /// starts and stops the service with it and GRKIT_HOST_WLAN.
    CONTROL_SERVICE: 4;
    CONTROL_PRODUCER_PADDING: 24;
    CONTROL_CLAIMED: 4;
    CONTROL_READY: 4;
    CONTROL_RESULT: 4;
    CONTROL_RESPONSE_LENGTH: 4;
    CONTROL_RESPONSE_CHECKSUM: 4;
    CONTROL_CONSUMER_PADDING: 44;
    CONTROL_PAYLOAD: CONTROL_PAYLOAD_BYTES;
    CONTROL_RESPONSE: CONTROL_PAYLOAD_BYTES;
});
pub const CONTROL_WORDS: usize = CONTROL_SIZE / 4;
/// CONTROL_SERVICE while the Wi-Fi service runs.
pub const CONTROL_SERVICE_RUNNING: u32 = 1;
