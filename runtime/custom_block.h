#ifndef GRKIT_CUSTOM_BLOCK_H
#define GRKIT_CUSTOM_BLOCK_H
/* The GR IV custom Image Control block on a GR III. The includer defines u8,
   s16, and u32. The code uses no library function, so the RTOS and Linux can
   both use it.

   A GR IV Custom slot holds a 56-byte block: byte 0 is the version (1),
   byte 2 the effect, and from byte 4 a (tag, value) pair for each parameter
   of the effect, in a fixed order. The rest is zero. The GR IV decoder reads
   the block all or nothing, and does not check byte 1, byte 3 (the pair
   count), or the bytes after the pairs. custom_block_decode does the same, and
   also rejects each value that a GR III Custom set cannot hold:

   - The effects that GR III does not have: Cinema Yellow, Cinema Green, and
     the effects of GR IV Monochrome.
   - Grain on Positive Film, which has no grain on GR III.
   - A grain size other than Small. The native grain of GR III is one random
     value per pixel, which is the Small size of GR IV.
   - Bleach Bypass toning 0 and HDR Tone strength 0. The GR IV decoder
     accepts them, but they have no GR IV or GR III value label.

   A GR III Custom set (CustomSet) holds the type and the parameters with the
   values that the UserData settings of GR III store. GR IV keeps the filter
   values once for each slot, as Custom 1 of the filter effect. GR III keeps
   four groups (Custom 1 to 4), so a write sets the filter effect to Custom 1
   and writes the values of group 1. A read reports any of Custom 1 to 4 as
   on, with the values of that group, as GR IV does. */

#define CUSTOM_BLOCK_SIZE 56U
#define CUSTOM_BLOCK_VERSION 1U

/* GR III types: the stored values of StillImageControlNo<n>. */
#define CUSTOM_STANDARD 11U
#define CUSTOM_VIVID 12U
#define CUSTOM_MONOTONE 13U
#define CUSTOM_SOFT_MONOTONE 14U
#define CUSTOM_HARD_MONOTONE 15U
#define CUSTOM_HIGH_CONTRAST_BW 16U
#define CUSTOM_POSITIVE_FILM 17U
#define CUSTOM_BLEACH_BYPASS 18U
#define CUSTOM_RETRO 19U
#define CUSTOM_HDR_TONE 20U
#define CUSTOM_CROSS_PROCESS 21U
#define CUSTOM_NEGATIVE_FILM 22U

/* GR III parameters. The toning of a type is one parameter: ColorToneMonotone,
   ColorToneBleechBypass, ColorToneRetro, ColorToneHdrIntense, or
   ColorToneCrossProcess, by type. HDR is HdrIntense, FILTER is
   FilterEffectCustom, and GRAIN is Particle. */
enum {
    CUSTOM_SATURATION, CUSTOM_HUE, CUSTOM_KEY, CUSTOM_CONTRAST, CUSTOM_HIGHLIGHT,
    CUSTOM_SHADOW, CUSTOM_SHARPNESS, CUSTOM_SHADING, CUSTOM_CLARITY, CUSTOM_TONE,
    CUSTOM_HDR, CUSTOM_FILTER, CUSTOM_GRAIN, CUSTOM_PARAMETERS
};

/* Stored values. A level parameter is M4..P4 = 0..8. Grain is Off 0, High 1,
   Mid 2, Low 3. Filter is Off 0, Custom 1..4 = 1..4. */
#define CUSTOM_LEVEL_ZERO 4U
#define CUSTOM_GRAIN_OFF 0U
/* A filter value is the weight of one channel in percent. The GR III menu
   edits -200..200 in steps of 10, as GR IV does. The block can hold other
   values in that range: GR III keeps them, and its menu shows the step
   toward zero. */
#define CUSTOM_FILTER_LIMIT 200

typedef struct {
    u32 type;
    /* The value of each parameter of the type. The other entries are 0. */
    u8 value[CUSTOM_PARAMETERS];
    /* R, G, B of the filter group: group 1 after a decode; for an encode, the
       group that FILTER selects, or group 1 when it is off. */
    s16 filter[3];
} CustomSet;

/* The result of custom_block_decode. Each rejection gives ATT 0x80, as on GR IV. */
enum {
    CUSTOM_OK,
    CUSTOM_BAD_BLOCK,   /* the length, the version, a tag, or a value is not valid on GR IV */
    CUSTOM_NO_EFFECT,   /* a GR IV effect that GR III does not have */
    CUSTOM_NOT_HELD     /* a valid GR IV value that a GR III Custom set cannot hold */
};

/* Block tags. */
enum {
    TAG_SATURATION, TAG_HUE, TAG_KEY, TAG_CONTRAST, TAG_HIGHLIGHT, TAG_SHADOW,
    TAG_SHARPNESS, TAG_TONE_MONOTONE, TAG_TONE_CINEMA, TAG_TONE_BLEACH,
    TAG_TONE_RETRO, TAG_TONE_HDR, TAG_FILTER, TAG_R_HIGH, TAG_R_LOW, TAG_G_HIGH,
    TAG_G_LOW, TAG_B_HIGH, TAG_B_LOW, TAG_SHADING, TAG_CLARITY, TAG_GRAIN,
    TAG_GRAIN_SIZE, TAG_GRAIN_STRENGTH, TAG_HDR, TAG_TONE_CROSS
};

/* The tags of each effect, in block order, and the end mark. */
#define TAG_END 0xffU
#define TAGS_LEVELS TAG_SATURATION, TAG_HUE, TAG_KEY, TAG_CONTRAST, TAG_HIGHLIGHT, TAG_SHADOW, TAG_SHARPNESS
#define TAGS_MONO TAG_KEY, TAG_CONTRAST, TAG_HIGHLIGHT, TAG_SHADOW, TAG_SHARPNESS, TAG_TONE_MONOTONE
#define TAGS_FILTER TAG_FILTER, TAG_R_HIGH, TAG_R_LOW, TAG_G_HIGH, TAG_G_LOW, TAG_B_HIGH, TAG_B_LOW
#define TAGS_GRAIN TAG_GRAIN, TAG_GRAIN_SIZE, TAG_GRAIN_STRENGTH
static const u8 custom_tags_color[] = {TAGS_LEVELS, TAG_SHADING, TAG_CLARITY, TAG_END};
static const u8 custom_tags_mono[] = {TAGS_MONO, TAGS_FILTER, TAG_SHADING, TAG_CLARITY, TAGS_GRAIN, TAG_END};
static const u8 custom_tags_soft[] = {TAGS_MONO, TAGS_FILTER, TAG_SHADING, TAGS_GRAIN, TAG_END};
static const u8 custom_tags_hard[] = {TAGS_MONO, TAG_SHADING, TAG_CLARITY, TAGS_GRAIN, TAG_END};
static const u8 custom_tags_film[] = {TAGS_LEVELS, TAG_SHADING, TAG_CLARITY, TAGS_GRAIN, TAG_END};
static const u8 custom_tags_bleach[] = {TAGS_LEVELS, TAG_TONE_BLEACH, TAG_SHADING, TAG_CLARITY, TAG_END};
static const u8 custom_tags_retro[] = {TAGS_LEVELS, TAG_TONE_RETRO, TAG_SHADING, TAG_CLARITY, TAG_END};
static const u8 custom_tags_hdr[] = {TAG_SATURATION, TAG_HUE, TAG_TONE_HDR, TAG_HDR, TAG_END};
static const u8 custom_tags_cross[] = {TAGS_LEVELS, TAG_SHADING, TAG_CLARITY, TAG_TONE_CROSS, TAG_END};

/* The GR IV effect of each GR III type, in type order from CUSTOM_STANDARD.
   `grain` is 1 when the GR III type has a grain parameter. */
static const struct {
    u8 effect, grain;
    const u8 *tags;
} custom_types[] = {
    {0, 0, custom_tags_color},  /* Standard */
    {1, 0, custom_tags_color},  /* Vivid */
    {2, 1, custom_tags_mono},   /* Monotone */
    {3, 1, custom_tags_soft},   /* Soft Monotone */
    {4, 1, custom_tags_hard},   /* Hard Monotone */
    {5, 1, custom_tags_mono},   /* Hi-Contrast B&W */
    {6, 0, custom_tags_film},   /* Positive Film */
    {7, 0, custom_tags_bleach}, /* Bleach Bypass */
    {8, 0, custom_tags_retro},  /* Retro */
    {9, 0, custom_tags_hdr},    /* HDR Tone */
    {16, 0, custom_tags_cross}, /* Cross Processing */
    {13, 1, custom_tags_film},  /* Negative Film */
};
#define CUSTOM_TYPES (sizeof custom_types / sizeof custom_types[0])

/* The GR III parameter of a tag that holds one value, or CUSTOM_PARAMETERS. */
static u32 custom_parameter(u32 tag)
{
    static const u8 parameter[] = {
        CUSTOM_SATURATION, CUSTOM_HUE, CUSTOM_KEY, CUSTOM_CONTRAST, CUSTOM_HIGHLIGHT,
        CUSTOM_SHADOW, CUSTOM_SHARPNESS, CUSTOM_TONE, CUSTOM_PARAMETERS, CUSTOM_TONE,
        CUSTOM_TONE, CUSTOM_TONE, CUSTOM_FILTER, CUSTOM_PARAMETERS, CUSTOM_PARAMETERS,
        CUSTOM_PARAMETERS, CUSTOM_PARAMETERS, CUSTOM_PARAMETERS, CUSTOM_PARAMETERS,
        CUSTOM_SHADING, CUSTOM_CLARITY, CUSTOM_PARAMETERS, CUSTOM_PARAMETERS,
        CUSTOM_PARAMETERS, CUSTOM_HDR, CUSTOM_TONE
    };
    return tag < sizeof parameter ? parameter[tag] : CUSTOM_PARAMETERS;
}

/* A level tag carries M4..P4 as -4..4, which GR III stores as 0..8. */
static int custom_level_tag(u32 tag)
{
    return tag <= TAG_SHARPNESS || tag == TAG_TONE_RETRO || tag == TAG_SHADING || tag == TAG_CLARITY;
}

/* The largest value of an enumerated tag on GR IV. */
static u32 custom_tag_max(u32 tag)
{
    switch (tag) {
    case TAG_TONE_MONOTONE: return 5;
    case TAG_TONE_BLEACH: case TAG_HDR: return 3;
    case TAG_TONE_CINEMA: case TAG_TONE_HDR: case TAG_GRAIN_SIZE:
    case TAG_GRAIN_STRENGTH: case TAG_TONE_CROSS: return 2;
    default: return 1; /* TAG_FILTER, TAG_GRAIN */
    }
}

/* Decode a block into a GR III Custom set. `set` is complete only on CUSTOM_OK. */
static u32 custom_block_decode(const u8 *block, u32 length, CustomSet *set)
{
    u32 index, i, held = 1, grain = 0, size = 0, strength = 0;
    const u8 *tags = 0;
    if (length != CUSTOM_BLOCK_SIZE || block[0] > CUSTOM_BLOCK_VERSION) return CUSTOM_BAD_BLOCK;
    for (index = 0; index < CUSTOM_TYPES; ++index)
        if (custom_types[index].effect == block[2]) break;
    if (index == CUSTOM_TYPES) {
        /* Cinema Yellow and Green (14, 15), and the Monochrome effects (17 to 22). */
        u32 effect = block[2];
        return effect == 14 || effect == 15 || (effect >= 17 && effect <= 22) ? CUSTOM_NO_EFFECT
                                                                             : CUSTOM_BAD_BLOCK;
    }
    for (i = 0; i < CUSTOM_PARAMETERS; ++i) set->value[i] = 0;
    for (i = 0; i < 3; ++i) set->filter[i] = 0;
    set->type = CUSTOM_STANDARD + index;
    tags = custom_types[index].tags;
    for (i = 0; tags[i] != TAG_END; ++i) {
        u32 tag = tags[i], raw = block[5 + i * 2];
        if (block[4 + i * 2] != tag) return CUSTOM_BAD_BLOCK;
        if (tag >= TAG_R_HIGH && tag <= TAG_B_LOW) {
            /* A filter value: a signed 16-bit pair, high byte first. */
            if ((tag - TAG_R_HIGH) & 1) {
                s16 value = (s16)(block[3 + i * 2] << 8 | raw);
                if (value < -CUSTOM_FILTER_LIMIT || value > CUSTOM_FILTER_LIMIT) return CUSTOM_BAD_BLOCK;
                set->filter[(tag - TAG_R_HIGH) >> 1] = value;
            }
            continue;
        }
        if (custom_level_tag(tag)) {
            /* -4..4 as a signed byte. */
            raw = (raw + 4) & 0xff;
            if (raw > 8) return CUSTOM_BAD_BLOCK;
        } else if (raw > custom_tag_max(tag)) {
            return CUSTOM_BAD_BLOCK;
        }
        if (tag == TAG_GRAIN) grain = raw;
        else if (tag == TAG_GRAIN_SIZE) size = raw;
        else if (tag == TAG_GRAIN_STRENGTH) strength = raw;
        else {
            if ((tag == TAG_TONE_BLEACH || tag == TAG_HDR) && raw == 0) held = 0;
            set->value[custom_parameter(tag)] = (u8)raw;
        }
    }
    /* Grain: GR IV Low, Mid, High (0..2) is GR III Low 3, Mid 2, High 1. */
    if (grain) {
        if (!custom_types[index].grain || size != 0) held = 0;
        set->value[CUSTOM_GRAIN] = (u8)(3 - strength);
    }
    return held ? CUSTOM_OK : CUSTOM_NOT_HELD;
}

/* Encode a GR III Custom set as the GR IV encoder does: the header, the pairs,
   then zeros. The GR III setters do not check their values, so a set can hold
   a value out of range. Such a set, or a type that is not a GR III type, gives
   0 and no valid block. */
static u32 custom_block_encode(const CustomSet *set, u8 *block)
{
    u32 index = set->type - CUSTOM_STANDARD, i, pairs = 0, particle = set->value[CUSTOM_GRAIN];
    const u8 *tags;
    if (set->type < CUSTOM_STANDARD || index >= CUSTOM_TYPES) return 0;
    if (particle > 3 || (particle != CUSTOM_GRAIN_OFF && !custom_types[index].grain)) return 0;
    tags = custom_types[index].tags;
    for (i = 0; i < CUSTOM_BLOCK_SIZE; ++i) block[i] = 0;
    block[0] = CUSTOM_BLOCK_VERSION;
    block[2] = custom_types[index].effect;
    for (i = 0; tags[i] != TAG_END; ++i, ++pairs) {
        u32 tag = tags[i], value;
        if (tag >= TAG_R_HIGH && tag <= TAG_B_LOW) {
            s16 filter = set->filter[(tag - TAG_R_HIGH) >> 1];
            if (filter < -CUSTOM_FILTER_LIMIT || filter > CUSTOM_FILTER_LIMIT) return 0;
            value = (tag - TAG_R_HIGH) & 1 ? (u16)filter & 0xff : (u16)filter >> 8;
        } else if (tag == TAG_GRAIN) {
            value = particle != CUSTOM_GRAIN_OFF;
        } else if (tag == TAG_GRAIN_SIZE) {
            value = 0;
        } else if (tag == TAG_GRAIN_STRENGTH) {
            value = particle != CUSTOM_GRAIN_OFF ? 3 - particle : 0;
        } else {
            value = set->value[custom_parameter(tag)];
            if (custom_level_tag(tag)) {
                if (value > 8) return 0;
                value = (value - CUSTOM_LEVEL_ZERO) & 0xff;
            } else if (tag == TAG_FILTER) {
                /* Custom 1..4 are all on. */
                if (value > 4) return 0;
                value = value != 0;
            } else if (value > custom_tag_max(tag) ||
                       ((tag == TAG_TONE_BLEACH || tag == TAG_HDR) && value == 0)) {
                return 0;
            }
        }
        block[4 + i * 2] = (u8)tag;
        block[5 + i * 2] = (u8)value;
    }
    block[3] = (u8)pairs;
    return 1;
}

#endif
