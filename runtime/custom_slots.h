#ifndef GRKIT_CUSTOM_SLOTS_H
#define GRKIT_CUSTOM_SLOTS_H
/* CORE.CUSTOM_SLOTS: a phone writes and reads the native Custom sets 1 and 2
   of a GR III in the format of the GR IV Custom slots (custom_block.h). The
   includer defines u8, s16, u32, WORD, HALF, BYTE, and the family values
   CUSTOM_SLOT_* (runtime/targets/gr3.h).

   The Passphrase transport (custom_transport.h) carries the requests. A
   committed write is text:

     W<n><112 hex digits>      write the block to Custom set n (1 or 2)
     N<n><0 to 64 hex digits>  write the name of Custom set n

   The transport reads a set with custom_slot_read and a name from its header
   words. The setters of the camera do not check values, and a written set
   stays after GRKit is removed, so a write decodes the block with each check
   of custom_block_decode first.

   GR III has no name for a Custom set, so the core keeps the names, as GR IV
   keeps them in its settings: up to 32 bytes each, padded with zeros, in
   GRKIT_HOST_CUSTOM_NAMES. A name write counts a state change, and the
   resident loader saves the names in the state file.

   A write stores the fields of the type of the block, the filter group 1,
   and then the type. When set n is active, the App task selects it again,
   as the native effect property does, so the live view follows. The camera
   saves the record at a normal power-off, as it saves a change in the menu.
   In a U mode the record is the copy of that mode, which the camera replaces
   when the dial turns, so a write in a U mode is refused. */

#include "custom_block.h"

/* The property type of a string. */
#define CUSTOM_SLOTS_STRING 3U
/* A name: its bytes, and the header words of each name, three bytes each. */
#define CUSTOM_SLOTS_NAME_SIZE 32U
#define CUSTOM_SLOTS_NAME_WORDS 11U
_Static_assert(2 * CUSTOM_SLOTS_NAME_WORDS == GRKIT_HOST_CUSTOM_NAMES_WORDS, "Two names fill the header field");
_Static_assert(3 * CUSTOM_SLOTS_NAME_WORDS >= CUSTOM_SLOTS_NAME_SIZE, "A name fits its words");

/* Whether a module that uses CORE.CUSTOM_SLOTS is on. Enable of the module
   in the GRKit section of MENU turns the requests off and on at once
   (runtime/manage.h). */
#define CUSTOM_SLOTS_RUNNING(h) ((h)[GRKIT_HOST_CAPABILITIES / 4] & GRKIT_CORE_CUSTOM_SLOTS_BIT)

/* The codes of a refused request. */
#define CUSTOM_SLOTS_BAD_REQUEST 1U /* not a request of the format above */
#define CUSTOM_SLOTS_OFF 2U         /* no module that uses CORE.CUSTOM_SLOTS is on */
#define CUSTOM_SLOTS_U_MODE 3U      /* the dial is on a U mode */
#define CUSTOM_SLOTS_BAD_BLOCK 4U   /* custom_block_decode: CUSTOM_BAD_BLOCK */
#define CUSTOM_SLOTS_NO_EFFECT 5U   /* custom_block_decode: CUSTOM_NO_EFFECT */
#define CUSTOM_SLOTS_NOT_HELD 6U    /* custom_block_decode: CUSTOM_NOT_HELD */
#define CUSTOM_SLOTS_UNREADABLE 7U  /* the set holds a value out of range */
#define CUSTOM_SLOTS_NOT_APPLIED 8U /* the set is stored, but the App task did not select it */

/* The parts of a LinuxCommCommand and of its property. */
#define CUSTOM_SLOTS_COMMAND_SIZE_AT 0xcU /* param 0: the bytes of the property */
#define CUSTOM_SLOTS_COMMAND_VALUE_AT 0x10U /* param 1: the property */
#define CUSTOM_SLOTS_TEXT_AT 0x10U /* a string: its padded size at +0xc, then the text */

static const u16 custom_slot_fields[CUSTOM_TYPES][CUSTOM_PARAMETERS] = CUSTOM_SLOT_FIELDS;

/* The field of `parameter` of `type` in set n, or 0. */
static u32 custom_slot_field(u32 type, u32 parameter, u32 n)
{
    u32 field = custom_slot_fields[type - CUSTOM_STANDARD][parameter];
    if (!field) return 0;
    return field + n * (field >= CUSTOM_SLOT_LATE ? CUSTOM_SLOT_LATE_STRIDE : CUSTOM_SLOT_STRIDE);
}
/* R, G, or B (i) of filter group g (1 to 4) of set n. */
static u32 custom_slot_filter(u32 g, u32 i, u32 n)
{
    return CUSTOM_SLOT_FILTER + n * CUSTOM_SLOT_STRIDE + (g - 1) * 12 + i * 4;
}

static void custom_slot_write(u32 record, u32 n, const CustomSet *set)
{
    for (u32 parameter = 0; parameter < CUSTOM_PARAMETERS; ++parameter) {
        u32 field = custom_slot_field(set->type, parameter, n);
        if (field) BYTE(record, field) = set->value[parameter];
    }
    if (custom_slot_field(set->type, CUSTOM_FILTER, n))
        for (u32 i = 0; i < 3; ++i) HALF(record, custom_slot_filter(1, i, n)) = (u16)set->filter[i];
    BYTE(record, CUSTOM_SLOT_TYPE(n)) = (u8)set->type;
}

/* Read set n. Return 0 for a type that is not a GR III type. */
static u32 custom_slot_read(u32 record, u32 n, CustomSet *set)
{
    u32 type = BYTE(record, CUSTOM_SLOT_TYPE(n));
    if (type < CUSTOM_STANDARD || type >= CUSTOM_STANDARD + CUSTOM_TYPES) return 0;
    set->type = type;
    for (u32 parameter = 0; parameter < CUSTOM_PARAMETERS; ++parameter) {
        u32 field = custom_slot_field(type, parameter, n);
        set->value[parameter] = field ? BYTE(record, field) : 0;
    }
    /* The filter group that the set selects, or group 1 when the filter is
       off. A value out of range fails the encode. */
    u32 g = set->value[CUSTOM_FILTER], group = g >= 1 && g <= 4 ? g : 1;
    for (u32 i = 0; i < 3; ++i)
        set->filter[i] = custom_slot_field(type, CUSTOM_FILTER, n) ? (s16)HALF(record, custom_slot_filter(group, i, n)) : 0;
    return 1;
}

static u32 custom_slots_hex(u8 c)
{
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    return 16;
}

/* Run the write request at `text` of `length` characters. Return 0 or a
   CUSTOM_SLOTS_* code. */
static u32 custom_slots_run(volatile u32 *h, u32 ctx, u32 text, u32 length)
{
#define TEXT(i) BYTE(text, i)
    u32 n = length >= 2 ? TEXT(1) - '0' : 0, settings, record;
    if (n != 1 && n != 2) return CUSTOM_SLOTS_BAD_REQUEST;
    volatile u32 *name = h + GRKIT_HOST_CUSTOM_NAMES / 4 + (n - 1) * CUSTOM_SLOTS_NAME_WORDS;
    if (TEXT(0) == 'W' && length == 2 + 2 * CUSTOM_BLOCK_SIZE) {
        u8 block[CUSTOM_BLOCK_SIZE];
        CustomSet set;
        for (u32 i = 0; i < CUSTOM_BLOCK_SIZE; ++i) {
            u32 high = custom_slots_hex(TEXT(2 + 2 * i)), low = custom_slots_hex(TEXT(3 + 2 * i));
            if (high > 15 || low > 15) return CUSTOM_SLOTS_BAD_REQUEST;
            block[i] = (u8)(high << 4 | low);
        }
        if (!CUSTOM_SLOTS_RUNNING(h)) return CUSTOM_SLOTS_OFF;
        settings = CUSTOM_SLOTS_SETTINGS();
        if (CUSTOM_SLOT_BOX(settings)) return CUSTOM_SLOTS_U_MODE;
        switch (custom_block_decode(block, CUSTOM_BLOCK_SIZE, &set)) {
        case CUSTOM_OK: break;
        case CUSTOM_NO_EFFECT: return CUSTOM_SLOTS_NO_EFFECT;
        case CUSTOM_NOT_HELD: return CUSTOM_SLOTS_NOT_HELD;
        default: return CUSTOM_SLOTS_BAD_BLOCK;
        }
        record = CUSTOM_SLOT_RECORD(settings);
        custom_slot_write(record, n, &set);
        if (BYTE(record, CUSTOM_SLOT_ACTIVE) == n && !CUSTOM_SLOTS_SELECT(ctx, n, set.type))
            return CUSTOM_SLOTS_NOT_APPLIED;
        return 0;
    }
    if (TEXT(0) == 'N' && length >= 2 && length <= 2 + 2 * CUSTOM_SLOTS_NAME_SIZE && length % 2 == 0) {
        u8 bytes[CUSTOM_SLOTS_NAME_SIZE];
        for (u32 i = 0; i < CUSTOM_SLOTS_NAME_SIZE; ++i) {
            u32 high = 0, low = 0;
            if (2 + 2 * i < length) {
                high = custom_slots_hex(TEXT(2 + 2 * i));
                low = custom_slots_hex(TEXT(3 + 2 * i));
                if (high > 15 || low > 15) return CUSTOM_SLOTS_BAD_REQUEST;
            }
            bytes[i] = (u8)(high << 4 | low);
        }
        if (!CUSTOM_SLOTS_RUNNING(h)) return CUSTOM_SLOTS_OFF;
        for (u32 w = 0; w < CUSTOM_SLOTS_NAME_WORDS; ++w) {
            u32 value = 0;
            for (u32 b = 0; b < 3 && 3 * w + b < CUSTOM_SLOTS_NAME_SIZE; ++b) value |= (u32)bytes[3 * w + b] << (8 * b);
            name[w] = value;
        }
        GRKIT_STATE_CHANGED(h);
        return 0;
    }
    return CUSTOM_SLOTS_BAD_REQUEST;
#undef TEXT
}

#endif
