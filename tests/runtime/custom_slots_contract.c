/* CORE.CUSTOM_SLOTS (runtime/custom_slots.h) with the GR III layout
   (runtime/targets/gr3.h), over a memory of its own. */
#include <assert.h>
#include <stdio.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned short u16;
typedef short s16;
typedef unsigned int u32;

/* Addresses are offsets in `memory`. */
static u8 memory[0x4000];
#define WORD(p, o) (*(volatile u32 *)(memory + (p) + (o)))
#define HALF(p, o) (*(volatile u16 *)(memory + (p) + (o)))
#define BYTE(p, o) (*(volatile u8 *)(memory + (p) + (o)))
#define SETTINGS 0x100U
#define BOX 0x200U
#define RECORD 0x1000U
#define RECORD_SIZE 0xb26U
#define REQUEST 0x3100U
#define CTX 0x1234U

static u32 selects, select_ctx, select_set, select_type, select_ok = 1;
#define CUSTOM_SLOTS_SETTINGS() SETTINGS
static u32 select_set_type(u32 ctx, u32 n, u32 type)
{
    ++selects;
    select_ctx = ctx;
    select_set = n;
    select_type = type;
    return select_ok;
}
#define CUSTOM_SLOTS_SELECT select_set_type

#include "abi.h"
#include "targets/gr3.h"
#include "custom_slots.h"

static u32 header[GRKIT_HOST_HEADER_SIZE / 4];

static void reset(void)
{
    memset(memory, 0, sizeof memory);
    memset(header, 0, sizeof header);
    WORD(SETTINGS, 0) = BOX;
    WORD(SETTINGS, 4) = RECORD;
    /* A module that uses CORE.CUSTOM_SLOTS is on, and other capabilities run. */
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_CUSTOM_SLOTS_BIT | GRKIT_CORE_GRAIN_BIT;
    selects = 0;
    select_ok = 1;
}


/* Run the write request `text`, as the transport does after a commit.
   Return 0 or the code. */
static u32 send(const char *text)
{
    u32 length = (u32)strlen(text);
    memset(memory + REQUEST, 0xee, 0x100);
    memcpy(memory + REQUEST, text, length);
    return custom_slots_run(header, CTX, REQUEST, length);
}

static void done(u32 code) { assert(code == 0); }

static void refused(u32 code, u32 expected) { assert(code == expected); }

static char request[2 + 2 * CUSTOM_BLOCK_SIZE + 1];

static const char *write_request(u32 n, const u8 *block)
{
    request[0] = 'W';
    request[1] = (char)('0' + n);
    for (u32 i = 0; i < CUSTOM_BLOCK_SIZE; ++i) snprintf(request + 2 + 2 * i, 3, "%02X", block[i]);
    return request;
}

/* A set of each type with values that differ from 0 and from each other. */
static CustomSet sample(u32 type)
{
    CustomSet set;
    u32 index = type - CUSTOM_STANDARD;
    const u8 *tags = custom_types[index].tags;
    memset(&set, 0, sizeof set);
    set.type = type;
    for (u32 i = 0; tags[i] != TAG_END; ++i) {
        u32 tag = tags[i], parameter = custom_parameter(tag);
        if (parameter == CUSTOM_PARAMETERS) continue;
        set.value[parameter] = (u8)(custom_level_tag(tag) ? 1 + i % 8 : custom_tag_max(tag));
    }
    if (custom_types[index].grain) set.value[CUSTOM_GRAIN] = 2;
    if (set.value[CUSTOM_FILTER]) {
        set.filter[0] = -200;
        set.filter[1] = 150;
        set.filter[2] = -10;
    }
    return set;
}


/* Read set n as the transport does. */
static u32 read_set(u32 n, u8 *block)
{
    CustomSet set;
    if (!custom_slot_read(CUSTOM_SLOT_RECORD(CUSTOM_SLOTS_SETTINGS()), n, &set) || !custom_block_encode(&set, block))
        return CUSTOM_SLOTS_UNREADABLE;
    return 0;
}

/* Read the name of set n from its header words. */
static void read_name(u32 n, u8 *name)
{
    const u32 *words = header + GRKIT_HOST_CUSTOM_NAMES / 4 + (n - 1) * CUSTOM_SLOTS_NAME_WORDS;
    for (u32 i = 0; i < CUSTOM_SLOTS_NAME_SIZE; ++i) name[i] = (u8)(words[i / 3] >> (8 * (i % 3)));
}

static char name_text[2 + 2 * (CUSTOM_SLOTS_NAME_SIZE + 1) + 1];

static const char *name_request(u32 n, const u8 *name, u32 length)
{
    name_text[0] = 'N';
    name_text[1] = (char)('0' + n);
    name_text[2] = 0;
    for (u32 i = 0; i < length; ++i) snprintf(name_text + 2 + 2 * i, 3, "%02X", name[i]);
    return name_text;
}

int main(void)
{
    /* The fields of each type are the parameters of its GR IV effect, and
       grain where GR III has it. No two fields share a byte or a flag byte,
       and each lies in the record. */
    {
        static u8 owner[RECORD_SIZE + 1];
        for (u32 index = 0; index < CUSTOM_TYPES; ++index) {
            u32 expected = custom_types[index].grain ? 1U << CUSTOM_GRAIN : 0;
            for (u32 i = 0; custom_types[index].tags[i] != TAG_END; ++i) {
                u32 parameter = custom_parameter(custom_types[index].tags[i]);
                if (parameter < CUSTOM_PARAMETERS) expected |= 1U << parameter;
            }
            for (u32 parameter = 0; parameter < CUSTOM_PARAMETERS; ++parameter)
                assert(!!custom_slot_fields[index][parameter] == !!(expected >> parameter & 1));
            for (u32 n = 0; n <= 2; ++n)
                for (u32 parameter = 0; parameter < CUSTOM_PARAMETERS; ++parameter) {
                    u32 field = custom_slot_field(CUSTOM_STANDARD + index, parameter, n);
                    if (!field) continue;
                    assert(field + 1 < RECORD_SIZE && !owner[field] && !owner[field + 1]);
                    owner[field] = owner[field + 1] = 1;
                }
        }
        for (u32 n = 0; n <= 2; ++n) {
            assert(!owner[CUSTOM_SLOT_TYPE(n)] && !owner[CUSTOM_SLOT_TYPE(n) + 1]);
            owner[CUSTOM_SLOT_TYPE(n)] = owner[CUSTOM_SLOT_TYPE(n) + 1] = 1;
            for (u32 g = 1; g <= 4; ++g)
                for (u32 i = 0; i < 3; ++i) {
                    u32 field = custom_slot_filter(g, i, n);
                    assert(field + 1 < RECORD_SIZE && !owner[field] && !owner[field + 1]);
                    owner[field] = owner[field + 1] = 1;
                }
        }
        assert(!owner[CUSTOM_SLOT_ACTIVE]);
    }

    /* A write of each type stores its fields, filter group 1, and the type in
       its set, and nothing else. The set is not active, so the App task
       does not select it. A read gives the same block back. */
    for (u32 type = CUSTOM_STANDARD; type < CUSTOM_STANDARD + CUSTOM_TYPES; ++type)
        for (u32 n = 1; n <= 2; ++n) {
            static u8 before[RECORD_SIZE];
            u8 block[CUSTOM_BLOCK_SIZE], back[CUSTOM_BLOCK_SIZE];
            CustomSet set = sample(type);
            reset();
            memcpy(before, memory + RECORD, RECORD_SIZE);
            assert(custom_block_encode(&set, block));
            done(send(write_request(n, block)));
            assert(selects == 0);
            assert(BYTE(RECORD, CUSTOM_SLOT_TYPE(n)) == type);
            for (u32 parameter = 0; parameter < CUSTOM_PARAMETERS; ++parameter) {
                u32 field = custom_slot_field(type, parameter, n);
                if (field) assert(BYTE(RECORD, field) == set.value[parameter] && BYTE(RECORD, field + 1) == 0);
            }
            for (u32 b = 0; b < RECORD_SIZE; ++b) {
                int written = b == CUSTOM_SLOT_TYPE(n);
                for (u32 parameter = 0; parameter < CUSTOM_PARAMETERS; ++parameter)
                    written |= b == custom_slot_field(type, parameter, n);
                if (custom_slot_field(type, CUSTOM_FILTER, n))
                    for (u32 i = 0; i < 3; ++i) written |= b - custom_slot_filter(1, i, n) < 2;
                if (!written) assert(memory[RECORD + b] == before[b]);
            }
            assert(read_set(n, back) == 0 && !memcmp(back, block, sizeof block));
        }

    /* Filter group 1 holds the filter values of the block. A read reports
       the group that the set selects. */
    reset();
    {
        CustomSet set = sample(CUSTOM_HIGH_CONTRAST_BW);
        u8 block[CUSTOM_BLOCK_SIZE], back[CUSTOM_BLOCK_SIZE];
        custom_block_encode(&set, block);
        done(send(write_request(1, block)));
        assert((s16)HALF(RECORD, custom_slot_filter(1, 0, 1)) == -200);
        assert((s16)HALF(RECORD, custom_slot_filter(1, 1, 1)) == 150);
        assert((s16)HALF(RECORD, custom_slot_filter(1, 2, 1)) == -10);
        HALF(RECORD, custom_slot_filter(3, 0, 1)) = (u16)90;
        HALF(RECORD, custom_slot_filter(3, 1, 1)) = (u16)20;
        HALF(RECORD, custom_slot_filter(3, 2, 1)) = (u16)-10;
        BYTE(RECORD, custom_slot_field(CUSTOM_HIGH_CONTRAST_BW, CUSTOM_FILTER, 1)) = 3;
        assert(read_set(1, back) == 0);
        assert(custom_block_decode(back, sizeof back, &set) == CUSTOM_OK && set.value[CUSTOM_FILTER] == 1);
        assert(set.filter[0] == 90 && set.filter[1] == 20 && set.filter[2] == -10);
    }

    /* A write of the active set has the App task select it again with its
       type. If that fails, the set is stored, and the write is refused. */
    reset();
    BYTE(RECORD, CUSTOM_SLOT_ACTIVE) = 2;
    {
        CustomSet set = sample(CUSTOM_POSITIVE_FILM);
        u8 block[CUSTOM_BLOCK_SIZE];
        custom_block_encode(&set, block);
        done(send(write_request(1, block)));
        assert(selects == 0);
        done(send(write_request(2, block)));
        assert(selects == 1 && select_ctx == CTX && select_set == 2 && select_type == CUSTOM_POSITIVE_FILM);
        select_ok = 0;
        BYTE(RECORD, CUSTOM_SLOT_TYPE(2)) = 0;
        refused(send(write_request(2, block)), CUSTOM_SLOTS_NOT_APPLIED);
        assert(BYTE(RECORD, CUSTOM_SLOT_TYPE(2)) == CUSTOM_POSITIVE_FILM);
    }

    /* A refused write changes nothing. */
    {
        static u8 before[RECORD_SIZE];
        CustomSet set = sample(CUSTOM_MONOTONE);
        u8 block[CUSTOM_BLOCK_SIZE];
        custom_block_encode(&set, block);
        reset();
        memcpy(before, memory + RECORD, RECORD_SIZE);
        header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_GRAIN_BIT;
        refused(send(write_request(1, block)), CUSTOM_SLOTS_OFF);
        header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_CUSTOM_SLOTS_BIT | GRKIT_CORE_GRAIN_BIT;
        BYTE(BOX, 0) = 1;
        refused(send(write_request(1, block)), CUSTOM_SLOTS_U_MODE);
        BYTE(BOX, 0) = 0;
        refused(send(write_request(3, block)), CUSTOM_SLOTS_BAD_REQUEST);
        block[2] = 14; /* Cinema Yellow */
        refused(send(write_request(1, block)), CUSTOM_SLOTS_NO_EFFECT);
        block[2] = 2;
        block[0] = 2;
        refused(send(write_request(1, block)), CUSTOM_SLOTS_BAD_BLOCK);
        block[0] = 1;
        /* Grain on with the Medium size. */
        custom_block_encode(&set, block);
        block[5 + 2 * 16] = 1; /* pair 16 is the grain size */
        refused(send(write_request(1, block)), CUSTOM_SLOTS_NOT_HELD);
        custom_block_encode(&set, block);
        write_request(1, block);
        request[10] = 'G';
        refused(send(request), CUSTOM_SLOTS_BAD_REQUEST);
        request[sizeof request - 2] = 0;
        refused(send(request), CUSTOM_SLOTS_BAD_REQUEST);
        refused(send("X1"), CUSTOM_SLOTS_BAD_REQUEST);
        refused(send(""), CUSTOM_SLOTS_BAD_REQUEST);
        assert(!memcmp(before, memory + RECORD, RECORD_SIZE) && selects == 0);
        /* Lowercase hex is a valid request. */
        write_request(1, block);
        for (char *p = request + 2; *p; ++p)
            if (*p >= 'A' && *p <= 'F') *p = (char)(*p - 'A' + 'a');
        done(send(request));
    }

    /* Reads: a set of no GR III type, or with a value out of range, cannot
       be read. */
    reset();
    {
        u8 back[CUSTOM_BLOCK_SIZE];
        assert(read_set(1, back) == CUSTOM_SLOTS_UNREADABLE);
        BYTE(RECORD, CUSTOM_SLOT_TYPE(1)) = CUSTOM_STANDARD;
        for (u32 parameter = 0; parameter < CUSTOM_PARAMETERS; ++parameter) {
            u32 field = custom_slot_field(CUSTOM_STANDARD, parameter, 1);
            if (field) BYTE(RECORD, field) = 4;
        }
        assert(read_set(1, back) == 0);
        BYTE(RECORD, custom_slot_field(CUSTOM_STANDARD, CUSTOM_SATURATION, 1)) = 9;
        assert(read_set(1, back) == CUSTOM_SLOTS_UNREADABLE);
        /* A read in a U mode reads the record of that mode. */
        BYTE(RECORD, custom_slot_field(CUSTOM_STANDARD, CUSTOM_SATURATION, 1)) = 4;
        BYTE(BOX, 0) = 2;
        assert(read_set(1, back) == 0);
    }

    /* Names: up to 32 bytes, padded with zeros, three bytes in each header
       word. A write counts a state change. */
    reset();
    {
        u8 name[CUSTOM_SLOTS_NAME_SIZE], other[CUSTOM_SLOTS_NAME_SIZE], zero[CUSTOM_SLOTS_NAME_SIZE] = {0};
        u32 changes = header[GRKIT_HOST_STATE_CHANGES / 4];
        read_name(1, name);
        assert(!memcmp(name, zero, sizeof name));
        done(send(name_request(1, (const u8 *)"Retro", 5)));
        assert(header[GRKIT_HOST_STATE_CHANGES / 4] == changes + 1);
        assert(header[GRKIT_HOST_CUSTOM_NAMES / 4] == ('R' | 'e' << 8 | 't' << 16));
        assert(header[GRKIT_HOST_CUSTOM_NAMES / 4 + 1] == ('r' | 'o' << 8));
        read_name(1, name);
        assert(!memcmp(name, "Retro", 6) && !memcmp(name + 5, zero, 27));
        read_name(2, name);
        assert(!memcmp(name, zero, sizeof name));
        /* All 32 bytes, any value, for set 2. Set 1 keeps its name. */
        for (u32 i = 0; i < sizeof other; ++i) other[i] = (u8)(0xff - 7 * i);
        done(send(name_request(2, other, sizeof other)));
        read_name(2, name);
        assert(!memcmp(name, other, sizeof name));
        assert(header[GRKIT_HOST_CUSTOM_NAMES / 4 + 2 * CUSTOM_SLOTS_NAME_WORDS - 1] ==
               (u32)(other[30] | other[31] << 8));
        read_name(1, name);
        assert(!memcmp(name, "Retro", 5));
        /* Refused writes keep the name and count no change. */
        changes = header[GRKIT_HOST_STATE_CHANGES / 4];
        refused(send(name_request(1, other, sizeof other + 1)), CUSTOM_SLOTS_BAD_REQUEST);
        refused(send("N1abc"), CUSTOM_SLOTS_BAD_REQUEST);
        refused(send("N1zz"), CUSTOM_SLOTS_BAD_REQUEST);
        refused(send("N3"), CUSTOM_SLOTS_BAD_REQUEST);
        refused(send("R10"), CUSTOM_SLOTS_BAD_REQUEST);
        refused(send("M10"), CUSTOM_SLOTS_BAD_REQUEST);
        header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_GRAIN_BIT;
        refused(send(name_request(1, other, 3)), CUSTOM_SLOTS_OFF);
        header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_CUSTOM_SLOTS_BIT | GRKIT_CORE_GRAIN_BIT;
        assert(header[GRKIT_HOST_STATE_CHANGES / 4] == changes);
        read_name(1, name);
        assert(!memcmp(name, "Retro", 5));
        /* A U mode does not hold the names, so a write there is done. An
           empty name clears it. Lowercase hex is valid. */
        BYTE(BOX, 0) = 2;
        done(send("N1"));
        read_name(1, name);
        assert(!memcmp(name, zero, sizeof name));
        done(send("N16162"));
        read_name(1, name);
        assert(!memcmp(name, "ab", 2) && !memcmp(name + 2, zero, 30));
        assert(selects == 0 && header[GRKIT_HOST_STATE_CHANGES / 4] == changes + 2);
    }
    return 0;
}
