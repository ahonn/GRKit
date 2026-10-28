/* The GR IV custom Image Control block on a GR III (runtime/custom_block.h). */
#include <assert.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned short u16;
typedef short s16;
typedef unsigned int u32;
#include "custom_block.h"

static u8 block[CUSTOM_BLOCK_SIZE];

/* A block of `effect` with the pairs of `tags` and `values`, as the GR IV
   encoder writes it. */
static void build(u32 effect, const u8 *tags, const int *values)
{
    u32 i;
    memset(block, 0, sizeof block);
    block[0] = CUSTOM_BLOCK_VERSION;
    block[2] = (u8)effect;
    for (i = 0; tags[i] != TAG_END; ++i) {
        block[4 + i * 2] = tags[i];
        block[5 + i * 2] = (u8)values[i];
    }
    block[3] = (u8)i;
}

static u32 decode(CustomSet *set)
{
    return custom_block_decode(block, sizeof block, set);
}

/* The pair index of `tag` in the block. */
static u32 pair(u32 tag)
{
    for (u32 i = 0;; ++i) {
        assert(4 + i * 2 < CUSTOM_BLOCK_SIZE);
        if (block[4 + i * 2] == tag) return i;
    }
}

static void set_pair(u32 tag, int value)
{
    block[5 + pair(tag) * 2] = (u8)value;
}

static int sets_equal(const CustomSet *a, const CustomSet *b)
{
    return a->type == b->type && !memcmp(a->value, b->value, sizeof a->value) &&
           !memcmp(a->filter, b->filter, sizeof a->filter);
}

/* A set of each type with values that differ from 0 and from each other. */
static CustomSet sample(u32 type)
{
    CustomSet set;
    u32 index = type - CUSTOM_STANDARD, i;
    const u8 *tags = custom_types[index].tags;
    memset(&set, 0, sizeof set);
    set.type = type;
    for (i = 0; tags[i] != TAG_END; ++i) {
        u32 tag = tags[i], parameter = custom_parameter(tag);
        if (parameter == CUSTOM_PARAMETERS) continue;
        set.value[parameter] = (u8)(custom_level_tag(tag) ? (i % 9) : custom_tag_max(tag));
    }
    if (custom_types[index].grain) set.value[CUSTOM_GRAIN] = 2;
    if (set.value[CUSTOM_FILTER]) {
        set.filter[0] = -200;
        set.filter[1] = 200;
        set.filter[2] = -1;
    }
    return set;
}

int main(void)
{
    CustomSet set, decoded;
    u32 type;

    /* Each GR III type encodes to the block of its GR IV effect and decodes
       back to the same set. */
    for (type = CUSTOM_STANDARD; type <= CUSTOM_NEGATIVE_FILM; ++type) {
        u32 count = 0;
        set = sample(type);
        assert(custom_block_encode(&set, block));
        assert(block[0] == 1 && block[1] == 0 && block[2] == custom_types[type - CUSTOM_STANDARD].effect);
        while (custom_types[type - CUSTOM_STANDARD].tags[count] != TAG_END) ++count;
        assert(block[3] == count);
        for (u32 i = 4 + count * 2; i < CUSTOM_BLOCK_SIZE; ++i) assert(block[i] == 0);
        assert(decode(&decoded) == CUSTOM_OK);
        assert(sets_equal(&set, &decoded));
    }

    /* Standard with each level at its edges: -4 is M4 (0), 0 is OFF (4), and
       4 is P4 (8). The toning of a type and its values are one parameter. */
    {
        static const int values[] = {-4, 4, 0, 1, -1, 2, -2, 3, -3};
        build(0, custom_tags_color, values);
        assert(decode(&set) == CUSTOM_OK && set.type == CUSTOM_STANDARD);
        assert(set.value[CUSTOM_SATURATION] == 0 && set.value[CUSTOM_HUE] == 8);
        assert(set.value[CUSTOM_KEY] == 4 && set.value[CUSTOM_CONTRAST] == 5);
        assert(set.value[CUSTOM_HIGHLIGHT] == 3 && set.value[CUSTOM_SHADOW] == 6);
        assert(set.value[CUSTOM_SHARPNESS] == 2 && set.value[CUSTOM_SHADING] == 7);
        assert(set.value[CUSTOM_CLARITY] == 1);
        assert(set.value[CUSTOM_TONE] == 0 && set.value[CUSTOM_GRAIN] == 0);
    }

    /* The effects map to the GR III types. */
    {
        static const u8 effects[] = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 16, 13};
        for (type = 0; type < 12; ++type) {
            set = sample(CUSTOM_STANDARD + type);
            assert(custom_block_encode(&set, block) && block[2] == effects[type]);
        }
    }

    /* The decoder does not check byte 1, the pair count, or the bytes after
       the pairs. Version 0 is accepted, as on GR IV. */
    set = sample(CUSTOM_MONOTONE);
    custom_block_encode(&set, block);
    block[0] = 0;
    block[1] = 0x5a;
    block[3] = 0;
    block[CUSTOM_BLOCK_SIZE - 1] = 0xff;
    assert(decode(&decoded) == CUSTOM_OK && sets_equal(&set, &decoded));

    /* A block that is not valid on GR IV. */
    set = sample(CUSTOM_STANDARD);
    custom_block_encode(&set, block);
    assert(custom_block_decode(block, CUSTOM_BLOCK_SIZE - 1, &decoded) == CUSTOM_BAD_BLOCK);
    block[0] = 2;
    assert(decode(&decoded) == CUSTOM_BAD_BLOCK);
    block[0] = 1;
    for (type = 10; type <= 12; ++type) {
        block[2] = (u8)type;
        assert(decode(&decoded) == CUSTOM_BAD_BLOCK);
    }
    block[2] = 23;
    assert(decode(&decoded) == CUSTOM_BAD_BLOCK);
    block[2] = 0;
    block[4] = TAG_HUE; /* a tag out of order */
    assert(decode(&decoded) == CUSTOM_BAD_BLOCK);
    block[4] = TAG_SATURATION;
    set_pair(TAG_SATURATION, 5);
    assert(decode(&decoded) == CUSTOM_BAD_BLOCK);
    set_pair(TAG_SATURATION, -5);
    assert(decode(&decoded) == CUSTOM_BAD_BLOCK);
    set_pair(TAG_SATURATION, 4);
    assert(decode(&decoded) == CUSTOM_OK);

    /* The enumerated tags stop at their largest GR IV value. */
    {
        static const struct { u32 type, tag, max; } limits[] = {
            {CUSTOM_MONOTONE, TAG_TONE_MONOTONE, 5}, {CUSTOM_BLEACH_BYPASS, TAG_TONE_BLEACH, 3},
            {CUSTOM_HDR_TONE, TAG_TONE_HDR, 2},      {CUSTOM_HDR_TONE, TAG_HDR, 3},
            {CUSTOM_CROSS_PROCESS, TAG_TONE_CROSS, 2}, {CUSTOM_MONOTONE, TAG_FILTER, 1},
            {CUSTOM_MONOTONE, TAG_GRAIN, 1},         {CUSTOM_MONOTONE, TAG_GRAIN_SIZE, 2},
            {CUSTOM_MONOTONE, TAG_GRAIN_STRENGTH, 2},
        };
        for (u32 i = 0; i < sizeof limits / sizeof limits[0]; ++i) {
            set = sample(limits[i].type);
            custom_block_encode(&set, block);
            if (custom_types[limits[i].type - CUSTOM_STANDARD].grain)
                set_pair(TAG_GRAIN, 0); /* any size is then held */
            set_pair(limits[i].tag, (int)limits[i].max);
            assert(decode(&decoded) == CUSTOM_OK);
            set_pair(limits[i].tag, (int)limits[i].max + 1);
            assert(decode(&decoded) == CUSTOM_BAD_BLOCK);
        }
    }

    /* Retro toning is a level. */
    set = sample(CUSTOM_RETRO);
    custom_block_encode(&set, block);
    set_pair(TAG_TONE_RETRO, -4);
    assert(decode(&decoded) == CUSTOM_OK && decoded.value[CUSTOM_TONE] == 0);
    set_pair(TAG_TONE_RETRO, 5);
    assert(decode(&decoded) == CUSTOM_BAD_BLOCK);

    /* The filter values: signed 16-bit, high byte first, at most 200 apart
       from 0. A block selects Custom 1. */
    set = sample(CUSTOM_HIGH_CONTRAST_BW);
    custom_block_encode(&set, block);
    assert(block[5 + pair(TAG_R_HIGH) * 2] == 0xff && block[5 + pair(TAG_R_LOW) * 2] == 0x38);
    assert(block[5 + pair(TAG_G_HIGH) * 2] == 0x00 && block[5 + pair(TAG_G_LOW) * 2] == 0xc8);
    set_pair(TAG_G_LOW, 0xc9);
    assert(decode(&decoded) == CUSTOM_BAD_BLOCK);
    set_pair(TAG_G_LOW, 0xc8);
    set_pair(TAG_R_LOW, 0x37);
    assert(decode(&decoded) == CUSTOM_BAD_BLOCK);
    set_pair(TAG_R_LOW, 0x38);
    assert(decode(&decoded) == CUSTOM_OK && decoded.value[CUSTOM_FILTER] == 1);
    assert(decoded.filter[0] == -200 && decoded.filter[1] == 200 && decoded.filter[2] == -1);
    /* Filter off keeps the values of group 1. */
    set_pair(TAG_FILTER, 0);
    assert(decode(&decoded) == CUSTOM_OK && decoded.value[CUSTOM_FILTER] == 0 && decoded.filter[1] == 200);
    /* A read reports Custom 1 to 4 as on, with the values of that group. */
    for (u32 group = 1; group <= 4; ++group) {
        set.value[CUSTOM_FILTER] = (u8)group;
        assert(custom_block_encode(&set, block) && block[5 + pair(TAG_FILTER) * 2] == 1);
    }

    /* Grain: GR IV Low, Mid, High is GR III Low 3, Mid 2, High 1, with the
       Small size. Off holds any size and strength. */
    for (u32 strength = 0; strength <= 2; ++strength) {
        set = sample(CUSTOM_NEGATIVE_FILM);
        custom_block_encode(&set, block);
        set_pair(TAG_GRAIN, 1);
        set_pair(TAG_GRAIN_SIZE, 0);
        set_pair(TAG_GRAIN_STRENGTH, (int)strength);
        assert(decode(&decoded) == CUSTOM_OK && decoded.value[CUSTOM_GRAIN] == 3 - strength);
        assert(custom_block_encode(&decoded, block));
        assert(block[5 + pair(TAG_GRAIN) * 2] == 1 && block[5 + pair(TAG_GRAIN_SIZE) * 2] == 0);
        assert(block[5 + pair(TAG_GRAIN_STRENGTH) * 2] == strength);
    }
    set_pair(TAG_GRAIN, 0);
    set_pair(TAG_GRAIN_SIZE, 2);
    set_pair(TAG_GRAIN_STRENGTH, 2);
    assert(decode(&decoded) == CUSTOM_OK && decoded.value[CUSTOM_GRAIN] == CUSTOM_GRAIN_OFF);
    assert(custom_block_encode(&decoded, block));
    assert(block[5 + pair(TAG_GRAIN_SIZE) * 2] == 0 && block[5 + pair(TAG_GRAIN_STRENGTH) * 2] == 0);

    /* Values that a GR III Custom set cannot hold. */
    set_pair(TAG_GRAIN, 1);
    set_pair(TAG_GRAIN_SIZE, 1);
    assert(decode(&decoded) == CUSTOM_NOT_HELD);
    set = sample(CUSTOM_POSITIVE_FILM);
    custom_block_encode(&set, block);
    assert(block[5 + pair(TAG_GRAIN) * 2] == 0);
    set_pair(TAG_GRAIN, 1);
    assert(decode(&decoded) == CUSTOM_NOT_HELD);
    set = sample(CUSTOM_BLEACH_BYPASS);
    custom_block_encode(&set, block);
    set_pair(TAG_TONE_BLEACH, 0);
    assert(decode(&decoded) == CUSTOM_NOT_HELD);
    set = sample(CUSTOM_HDR_TONE);
    custom_block_encode(&set, block);
    set_pair(TAG_HDR, 0);
    assert(decode(&decoded) == CUSTOM_NOT_HELD);

    /* The GR IV effects that GR III does not have: Cinema Yellow and Green,
       and the Monochrome effects. */
    for (type = 14; type <= 22; ++type) {
        if (type == 16) continue;
        set = sample(CUSTOM_STANDARD);
        custom_block_encode(&set, block);
        block[2] = (u8)type;
        assert(decode(&decoded) == CUSTOM_NO_EFFECT);
    }

    /* A set with a value out of range gives no block. */
    set = sample(CUSTOM_STANDARD);
    set.value[CUSTOM_SATURATION] = 9;
    assert(!custom_block_encode(&set, block));
    set = sample(CUSTOM_POSITIVE_FILM);
    set.value[CUSTOM_GRAIN] = 2;
    assert(!custom_block_encode(&set, block));
    set = sample(CUSTOM_MONOTONE);
    set.value[CUSTOM_GRAIN] = 4;
    assert(!custom_block_encode(&set, block));
    set = sample(CUSTOM_MONOTONE);
    set.value[CUSTOM_FILTER] = 5;
    assert(!custom_block_encode(&set, block));
    set = sample(CUSTOM_MONOTONE);
    set.filter[2] = 201;
    assert(!custom_block_encode(&set, block));
    set = sample(CUSTOM_BLEACH_BYPASS);
    set.value[CUSTOM_TONE] = 0;
    assert(!custom_block_encode(&set, block));
    set = sample(CUSTOM_HDR_TONE);
    set.value[CUSTOM_HDR] = 4;
    assert(!custom_block_encode(&set, block));
    set.type = CUSTOM_STANDARD - 1;
    assert(!custom_block_encode(&set, block));
    set.type = CUSTOM_NEGATIVE_FILM + 1;
    assert(!custom_block_encode(&set, block));
    return 0;
}
