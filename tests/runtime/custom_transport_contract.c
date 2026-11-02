/* Native Passphrase transport over independent memory and the real Custom
   codec. The fake pool models ownership; firmware/I/O behavior needs devices. */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned short u16;
typedef short s16;
typedef unsigned int u32;
static u8 memory[0x5000];
#define WORD(p, o) (*(volatile u32 *)(memory + (p) + (o)))
#define HALF(p, o) (*(volatile u16 *)(memory + (p) + (o)))
#define BYTE(p, o) (*(volatile u8 *)(memory + (p) + (o)))
#define SETTINGS 0x100U
#define BOX 0x200U
#define RECORD 0x1000U
#define COMMAND 0x3000U
#define PROPERTY 0x3100U
#define ALLOCATION 0x3400U
#define COPIED_TEXT 0x3600U
#define CTX 0x1234U
static u32 released, results[4], result_calls, allocation_calls, allocation_bytes;
static u32 allocation_ok = 1, selects, select_ok = 1, epoch;
static uint64_t ticks;
#define CUSTOM_SLOTS_SETTINGS() SETTINGS
#define CUSTOM_SLOTS_RELEASE(property) (released = (property))
#define CUSTOM_SLOTS_RESULT(command, n, value) \
    (assert((command) == COMMAND && (n) < 4), results[n] = (value), ++result_calls)
static u32 select_set(u32 ctx, u32 n, u32 type)
{
    assert(ctx == CTX && (n == 1 || n == 2) && type >= 11);
    ++selects;
    return select_ok;
}
#define CUSTOM_SLOTS_SELECT select_set
#include "abi.h"
#include "targets/gr3.h"
#include "custom_slots.h"
static u32 allocate(u32 bytes)
{
    ++allocation_calls;
    allocation_bytes = bytes;
    return allocation_ok ? ALLOCATION : 0;
}
static u32 copy_text(const volatile u8 *text, u32 length)
{
    assert(length < 116);
    for (u32 i = 0; i < length; ++i) memory[COPIED_TEXT + i] = text[i];
    return COPIED_TEXT;
}
#define CUSTOM_TRANSPORT_ALLOC(bytes) allocate(bytes)
#define CUSTOM_TRANSPORT_TEXT_ADDRESS(bytes, length) copy_text(bytes, length)
#define CUSTOM_TRANSPORT_NOW(low, high) ((low) = (u32)ticks, (high) = (u32)(ticks >> 32))
#define CUSTOM_TRANSPORT_EPOCH(h) ((void)(h), epoch)
static u32 transport_storage[264 / 4];
#define CUSTOM_TRANSPORT_STATE (*(CustomTransportState *)transport_storage)
#include "custom_transport.h"
#define transport CUSTOM_TRANSPORT_STATE
static u32 header[GRKIT_HOST_HEADER_SIZE / 4];
static unsigned checks;
#define CHECK(condition) (assert(condition), ++checks)

/* Independent wire CRC with a published CCITT-FALSE check vector below. */
static unsigned crc16(const char *bytes, size_t length)
{
    unsigned crc = 0xffff;
    for (size_t i = 0; i < length; ++i) {
        crc ^= (unsigned)(unsigned char)bytes[i] << 8;
        for (unsigned j = 0; j < 8; ++j) {
            unsigned carry = crc & 0x8000;
            crc = (crc * 2) & 0xffff;
            if (carry) crc ^= 0x1021;
        }
    }
    return crc;
}

static void reset(void)
{
    memset(memory, 0, sizeof memory);
    memset(header, 0, sizeof header);
    memset(&transport, 0, sizeof transport);
    WORD(SETTINGS, 0) = BOX;
    WORD(SETTINGS, 4) = RECORD;
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_CUSTOM_SLOTS_BIT;
    ticks = 100;
    epoch = 1;
    selects = 0;
    select_ok = allocation_ok = 1;
    allocation_calls = 0;
}

static u32 send_as(u32 property, const char *text)
{
    u32 length = (u32)strlen(text), size = (length + 4) & ~3U;
    assert(length < 200);
    memset(memory + PROPERTY, 0xee, 0x200);
    WORD(PROPERTY, 0) = property;
    WORD(PROPERTY, 4) = CUSTOM_SLOTS_STRING;
    WORD(PROPERTY, 8) = 1;
    WORD(PROPERTY, 12) = size;
    memcpy(memory + PROPERTY + 16, text, length + 1);
    memset(memory + COMMAND, 0, 0x34);
    WORD(COMMAND, 12) = 16 + size;
    WORD(COMMAND, 16) = PROPERTY;
    released = result_calls = 0;
    return custom_transport_set(header, CTX, COMMAND);
}

static u32 get(u32 property)
{
    WORD(COMMAND, 12) = property;
    result_calls = released = 0;
    return custom_transport_get(header, CTX, COMMAND);
}

static char request[201];
static void format(u32 id, char action, u32 index, const char *body)
{
    int length = snprintf(request, sizeof request, "\x1bGRK1%08x%c%02x%s", id, action, index, body);
    assert(length >= 16 && length < (int)sizeof request);
}

static void verify_reply(u32 code, const char *data)
{
    CHECK(get(32) == 2 && released == 0 && result_calls == 4);
    CHECK(results[0] == 0 && results[1] == 0 && results[3] == ALLOCATION);
    CHECK(WORD(ALLOCATION, 0) == 32 && WORD(ALLOCATION, 4) == 3 && WORD(ALLOCATION, 8) == 1);
    char expected[40];
    memcpy(expected, request, 16);
    snprintf(expected + 16, sizeof expected - 16, "%02x%s", code, data ? data : "");
    size_t length = strlen(expected);
    unsigned crc = crc16(expected, length);
    snprintf(expected + length, sizeof expected - length, "%04x", crc);
    length += 4;
    u32 padded = ((u32)length + 4) & ~3U;
    CHECK(allocation_bytes == 16 + padded && results[2] == 16 + padded && WORD(ALLOCATION, 12) == padded);
    CHECK(memcmp(memory + ALLOCATION + 16, expected, length + 1) == 0);
    for (u32 i = (u32)length; i < padded; ++i) CHECK(BYTE(ALLOCATION, 16 + i) == 0);
}

static void request_code(u32 id, char action, u32 index, const char *body, u32 code)
{
    format(id, action, index, body);
    CHECK(send_as(32, request) == 2);
    CHECK(released == PROPERTY && result_calls == 4 && results[0] == 0);
    verify_reply(code, NULL);
}

static void close_transaction(u32 id)
{
    format(id, 'Z', 0, "");
    CHECK(send_as(32, request) == 2 && released == PROPERTY);
    CHECK(get(32) == 0 && transport.id == 0 && transport.pending == 0);
    CustomTransportState empty = {0};
    CHECK(memcmp(&transport, &empty, sizeof empty) == 0);
}

static char block_command[115];
static void valid_block(void)
{
    const char *hex = "0100080a00020100020003ff0400050006010a03130014";
    block_command[0] = 'W'; block_command[1] = '1';
    memset(block_command + 2, '0', 112);
    memcpy(block_command + 2, hex, strlen(hex));
    block_command[114] = 0;
}

static void stage(u32 id, const char *command)
{
    u32 length = (u32)strlen(command);
    for (u32 offset = 0; offset < length;) {
        char part[48];
        u32 count = length - offset < 47 ? length - offset : 47;
        memcpy(part, command + offset, count);
        part[count] = 0;
        request_code(id, offset ? 'A' : 'B', offset, part, 0);
        offset += count;
    }
}

static void commit(u32 id, const char *command, u32 expected)
{
    char crc[5];
    snprintf(crc, sizeof crc, "%04x", crc16(command, strlen(command)));
    request_code(id, 'X', (u32)strlen(command), crc, expected);
}

static void native_and_framing(void)
{
    reset();
    CHECK(crc16("123456789", 9) == 0x29b1);
    CHECK(custom_transport_crc((const u8 *)"123456789", 9) == 0x29b1);
    CHECK(send_as(31, "\x1bGRK100000001P00") == 0 && released == 0);
    CHECK(send_as(32, "native passphrase") == 0 && released == 0);
    CHECK(send_as(32, "\x1bGRK1") == 1 && released == PROPERTY && results[1] == 1);
    CHECK(send_as(32, "\x1bGRK100000000P00") == 1 && released == PROPERTY);
    CHECK(send_as(32, "\x1bGRK10000000zP00") == 1);
    CHECK(send_as(32, "\x1bGRK100000001Pgg") == 1);
    char overlong[65];
    memcpy(overlong, "\x1bGRK100000001B00", 16);
    memset(overlong + 16, 'a', 48);
    overlong[64] = 0;
    CHECK(send_as(32, overlong) == 1 && released == PROPERTY);
    format(1, 'P', 0, "");
    send_as(32, request);
    for (u32 i = 16; i < WORD(PROPERTY, 12); ++i) BYTE(PROPERTY, 16 + i) = 'a';
    CHECK(custom_transport_set(header, CTX, COMMAND) == 1 && results[1] == 1);
    request_code(1, 'P', 0, "", 0);
    CHECK(get(31) == 0 && transport.id == 1);
    CHECK(send_as(32, "x") == 0 && get(32) == 0 && transport.id == 0);
    request_code(2, 'P', 0, "", 0);
    CHECK(send_as(32, "") == 0 && transport.id == 0);
    /* A claimed string span cannot escape the serialized property. */
    format(3, 'P', 0, "");
    send_as(32, request);
    WORD(PROPERTY, 12) = 0x100;
    released = 0;
    CHECK(custom_transport_set(header, CTX, COMMAND) == 1 && released == PROPERTY);
    WORD(PROPERTY, 8) = 2;
    released = 0;
    CHECK(custom_transport_set(header, CTX, COMMAND) == 0 && released == 0 && !transport.id);
}

static void transactions_and_writes(void)
{
    reset();
    u8 before[0xb26];
    memcpy(before, memory + RECORD, sizeof before);
    request_code(1, 'P', 0, "", 0);
    u32 stamp = transport.tick_low;
    ++ticks;
    request_code(1, 'P', 0, "", 0);
    CHECK(transport.tick_low == stamp);
    request_code(2, 'P', 0, "", CUSTOM_TRANSPORT_BUSY);
    request_code(2, 'B', 0, "W1", CUSTOM_TRANSPORT_BUSY);
    request_code(2, 'Q', 0, "R1", CUSTOM_TRANSPORT_BUSY);
    request_code(2, 'Z', 0, "", CUSTOM_SLOTS_BAD_REQUEST);
    CHECK(transport.id == 1 && transport.tick_low == stamp);
    request_code(1, 'B', 0, "W1", 0);
    stamp = transport.tick_low;
    ++ticks;
    request_code(1, 'B', 0, "W1", 0);
    CHECK(transport.length == 2 && transport.tick_low == stamp);
    request_code(1, 'B', 0, "W2", CUSTOM_TRANSPORT_CONFLICT);
    request_code(1, 'A', 3, "aa", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(1, 'A', 1, "1aa", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(1, 'A', 2, "aa", 0);
    stamp = transport.tick_low;
    ++ticks;
    request_code(1, 'A', 2, "aa", 0);
    CHECK(transport.length == 4 && transport.tick_low == stamp);
    request_code(1, 'A', 2, "bb", CUSTOM_TRANSPORT_CONFLICT);
    request_code(1, 'P', 0, "", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(1, 'Q', 0, "R1", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(1, 'X', 4, "0000", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(1, 'A', 4, "aa", CUSTOM_SLOTS_BAD_REQUEST);
    CHECK(memcmp(before, memory + RECORD, sizeof before) == 0);
    close_transaction(1);

    valid_block();
    stage(3, block_command);
    CHECK(memcmp(before, memory + RECORD, sizeof before) == 0);
    request_code(3, 'A', 114, "a", CUSTOM_SLOTS_BAD_REQUEST);
    BYTE(RECORD, CUSTOM_SLOT_ACTIVE) = 1;
    commit(3, block_command, 0);
    CHECK(BYTE(RECORD, CUSTOM_SLOT_TYPE(1)) == CUSTOM_RETRO && selects == 1);
    stamp = transport.tick_low;
    ++ticks;
    commit(3, block_command, 0);
    CHECK(selects == 1 && transport.tick_low == stamp);
    request_code(3, 'B', 0, "N141", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(3, 'X', 114, "0000", CUSTOM_SLOTS_BAD_REQUEST);
    CHECK(selects == 1);
    close_transaction(3);

    /* A failed native apply is terminal too: do not apply again on a retry. */
    stage(4, block_command);
    select_ok = 0;
    commit(4, block_command, CUSTOM_SLOTS_NOT_APPLIED);
    CHECK(selects == 2);
    commit(4, block_command, CUSTOM_SLOTS_NOT_APPLIED);
    CHECK(selects == 2);
    close_transaction(4);
    select_ok = 1;

    /* Real codec refusals and incomplete transactions leave the native set. */
    memcpy(before, memory + RECORD, sizeof before);
    block_command[6] = '0'; block_command[7] = 'e'; /* Cinema Yellow. */
    stage(5, block_command);
    commit(5, block_command, CUSTOM_SLOTS_NO_EFFECT);
    CHECK(memcmp(before, memory + RECORD, sizeof before) == 0);
    close_transaction(5);
    stage(6, "W1");
    commit(6, "W1", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(6, 'A', 2, "a", CUSTOM_SLOTS_BAD_REQUEST);
    close_transaction(6);
    valid_block();
    BYTE(BOX, 0) = 1;
    stage(7, block_command);
    commit(7, block_command, CUSTOM_SLOTS_U_MODE);
    CHECK(memcmp(before, memory + RECORD, sizeof before) == 0);
    close_transaction(7);
    BYTE(BOX, 0) = 0;
}

static void snapshots_and_names(void)
{
    reset();
    valid_block();
    stage(1, block_command);
    commit(1, block_command, 0);
    close_transaction(1);
    request_code(2, 'Q', 1, "R1", CUSTOM_SLOTS_BAD_REQUEST);
    CHECK(transport.id == 0);
    format(2, 'Q', 0, "R1");
    CHECK(send_as(32, request) == 2);
    verify_reply(0, "0100080a00020100");
    u8 snapshot[56];
    memcpy(snapshot, transport.snapshot, sizeof snapshot);
    /* Changing the camera record after page zero cannot change later pages. */
    BYTE(RECORD, custom_slot_field(CUSTOM_RETRO, CUSTOM_CONTRAST, 1)) = 8;
    ++ticks;
    for (u32 page = 1; page < 7; ++page) {
        char hex[17];
        for (u32 i = 0; i < 8; ++i) snprintf(hex + 2 * i, 3, "%02x", snapshot[page * 8 + i]);
        format(2, 'Q', page, "R1");
        CHECK(send_as(32, request) == 2);
        verify_reply(0, hex);
    }
    u32 stamp = transport.tick_low;
    ++ticks;
    format(2, 'Q', 0, "R1");
    CHECK(send_as(32, request) == 2);
    verify_reply(0, "0100080a00020100");
    CHECK(transport.tick_low == stamp);
    request_code(2, 'Q', 0, "R2", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(2, 'Q', 0, "M1", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(2, 'Q', 7, "R1", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(2, 'B', 0, "N141", CUSTOM_SLOTS_BAD_REQUEST);
    close_transaction(2);

    stage(3, "N148656c6c6f");
    commit(3, "N148656c6c6f", 0);
    close_transaction(3);
    format(4, 'Q', 0, "M1");
    CHECK(send_as(32, request) == 2);
    verify_reply(0, "48656c6c6f000000");
    header[GRKIT_HOST_CUSTOM_NAMES / 4 + 3] = 0xffffff;
    format(4, 'Q', 1, "M1");
    CHECK(send_as(32, request) == 2);
    verify_reply(0, "0000000000000000");
    request_code(4, 'Q', 4, "M1", CUSTOM_SLOTS_BAD_REQUEST);
    close_transaction(4);
    stage(5, "N1");
    commit(5, "N1", 0);
    close_transaction(5);
    format(6, 'Q', 0, "M1");
    CHECK(send_as(32, request) == 2);
    verify_reply(0, "0000000000000000");
    close_transaction(6);
}

static void expiry_disable_and_pool(void)
{
    reset();
    request_code(1, 'P', 0, "", 0);
    allocation_ok = 0;
    CHECK(get(32) == 1 && results[0] == 1 && results[1] == 4 && !results[2] && !results[3]);
    CHECK(transport.pending && !released);
    allocation_ok = 1;
    CHECK(get(32) == 2 && !released);
    uint64_t start = ticks;
    ticks += CUSTOM_TRANSPORT_TIMEOUT_TICKS - 1;
    CHECK(get(32) == 2);
    request_code(1, 'P', 0, "", 0);
    CHECK(transport.tick_low == (u32)start);
    ++ticks;
    CHECK(get(32) == 0 && !transport.id);
    close_transaction(1); /* Cleanup after expiration must not create a reply. */
    close_transaction(1); /* Idle reset is idempotent. */

    /* Wrap the low word, then the whole 64-bit counter. */
    ticks = UINT64_C(0xfffffff0);
    request_code(2, 'P', 0, "", 0);
    ticks += CUSTOM_TRANSPORT_TIMEOUT_TICKS - 1;
    CHECK(get(32) == 2);
    ++ticks;
    CHECK(get(32) == 0);
    ticks = UINT64_MAX - 100;
    request_code(3, 'P', 0, "", 0);
    ticks += CUSTOM_TRANSPORT_TIMEOUT_TICKS - 1;
    CHECK(get(32) == 2);
    ++ticks;
    CHECK(get(32) == 0);

    ticks = 10;
    request_code(4, 'B', 0, "N141", 0);
    ++epoch; /* Off then On happened between requests. */
    CHECK(get(32) == 0 && !transport.id && !transport.length);
    request_code(4, 'X', 4, "0000", CUSTOM_SLOTS_BAD_REQUEST);
    request_code(5, 'P', 0, "", 0);
    header[GRKIT_HOST_CAPABILITIES / 4] = 0;
    CHECK(get(32) == 0 && !transport.id);
    format(5, 'P', 0, "");
    CHECK(send_as(32, request) == 0 && !released && !result_calls);
    header[GRKIT_HOST_CAPABILITIES / 4] = GRKIT_CORE_CUSTOM_SLOTS_BIT;
    request_code(6, 'P', 0, "", 0);
    close_transaction(6);

    /* Rejected initial requests have a bounded reply, not a reserved ID. */
    request_code(7, 'Q', 1, "M1", CUSTOM_SLOTS_BAD_REQUEST);
    CHECK(!transport.id && transport.pending);
    ticks += CUSTOM_TRANSPORT_TIMEOUT_TICKS;
    CHECK(get(32) == 0 && !transport.pending);
}

int main(void)
{
    native_and_framing();
    transactions_and_writes();
    snapshots_and_names();
    expiry_disable_and_pool();
    printf("Custom transport: %u assertions passed\n", checks);
    return 0;
}
