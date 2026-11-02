#ifndef GRKIT_CUSTOM_TRANSPORT_H
#define GRKIT_CUSTOM_TRANSPORT_H
/* Native Passphrase transport for CORE.CUSTOM_SLOTS. Include after
   custom_slots.h. The property task serializes requests, but does not identify
   the Linux caller: webapid can observe a pending response too.
   docs/custom-ble-protocol.md gives the frame format. */
#define CUSTOM_TRANSPORT_PROPERTY 32U
#define CUSTOM_TRANSPORT_HEADER 16U
#define CUSTOM_TRANSPORT_TEXT_MAX 63U
#define CUSTOM_TRANSPORT_STAGE_SIZE 116U
#define CUSTOM_TRANSPORT_RESPONSE_SIZE 40U
#define CUSTOM_TRANSPORT_BUSY 9U
#define CUSTOM_TRANSPORT_CONFLICT 10U
#ifndef CUSTOM_TRANSPORT_TIMEOUT_TICKS
#define CUSTOM_TRANSPORT_TIMEOUT_TICKS 1200000000U /* 30 s at 40 MHz */
#endif
#define CUSTOM_TRANSPORT_PROBE 1U
#define CUSTOM_TRANSPORT_WRITE 2U
#define CUSTOM_TRANSPORT_READ 3U
#define CUSTOM_TRANSPORT_COMMITTED 4U

typedef struct {
    u32 id;
    u32 epoch;
    u32 tick_low;
    u32 tick_high;
    u32 mode;
    u32 length;
    u32 snapshot_length;
    u32 query;
    u32 pages;
    u32 commit_crc;
    u32 commit_code;
    u32 pending;
    u32 response_length;
    u8 stage[CUSTOM_TRANSPORT_STAGE_SIZE];
    u8 snapshot[CUSTOM_BLOCK_SIZE];
    u8 response[CUSTOM_TRANSPORT_RESPONSE_SIZE];
} CustomTransportState;
_Static_assert(sizeof(CustomTransportState) == 264, "Custom transport state ABI");

#ifndef CUSTOM_TRANSPORT_TEXT_ADDRESS
#define CUSTOM_TRANSPORT_TEXT_ADDRESS(bytes, length) ((u32)(bytes))
#endif

static void custom_transport_clear(volatile CustomTransportState *s)
{
    volatile u8 *bytes = (volatile u8 *)s;
    for (u32 i = 0; i < sizeof(*s); ++i) bytes[i] = 0;
}

static u32 custom_transport_crc_byte(u32 crc, u8 byte)
{
    crc ^= (u32)byte << 8;
    for (u32 bit = 0; bit < 8; ++bit)
        crc = ((crc << 1) ^ ((crc & 0x8000U) ? 0x1021U : 0U)) & 0xffffU;
    return crc;
}

static u32 custom_transport_crc(const volatile u8 *bytes, u32 length)
{
    u32 crc = 0xffffU;
    for (u32 i = 0; i < length; ++i) crc = custom_transport_crc_byte(crc, bytes[i]);
    return crc;
}

static u32 custom_transport_number(u32 text, u32 offset, u32 length, u32 *value)
{
    u32 result = 0;
    for (u32 i = 0; i < length; ++i) {
        u32 digit = custom_slots_hex(BYTE(text, offset + i));
        if (digit > 15) return 0;
        result = (result << 4) | digit;
    }
    *value = result;
    return 1;
}

/* A modular two-word subtraction also handles rollover of the low word and
   of the complete counter. GET and retransmissions never extend the lease. */
static void custom_transport_sync(volatile CustomTransportState *s, volatile u32 *h,
                                  u32 low, u32 high)
{
    if (!CUSTOM_SLOTS_RUNNING(h)) {
        custom_transport_clear(s);
        return;
    }
    if (s->id || s->pending) {
        u32 elapsed_low = low - s->tick_low;
        u32 elapsed_high = high - s->tick_high - (low < s->tick_low);
        if (s->epoch != CUSTOM_TRANSPORT_EPOCH(h) || elapsed_high ||
            elapsed_low >= CUSTOM_TRANSPORT_TIMEOUT_TICKS) custom_transport_clear(s);
    }
}

static void custom_transport_begin(volatile CustomTransportState *s, volatile u32 *h,
                                   u32 id, u32 low, u32 high)
{
    custom_transport_clear(s);
    s->id = id;
    s->epoch = CUSTOM_TRANSPORT_EPOCH(h);
    s->mode = CUSTOM_TRANSPORT_PROBE;
    s->tick_low = low;
    s->tick_high = high;
}

static void custom_transport_result(u32 command, u32 failed, u32 code, u32 bytes, u32 property)
{
    CUSTOM_SLOTS_RESULT(command, 0, failed);
    CUSTOM_SLOTS_RESULT(command, 1, code);
    CUSTOM_SLOTS_RESULT(command, 2, bytes);
    CUSTOM_SLOTS_RESULT(command, 3, property);
}

static void custom_transport_reply(volatile CustomTransportState *s, u32 text,
                                   u32 code, u32 page, u32 data)
{
    static const u8 digits[] = "0123456789abcdef";
    for (u32 i = 0; i < CUSTOM_TRANSPORT_HEADER; ++i) s->response[i] = BYTE(text, i);
    u32 length = CUSTOM_TRANSPORT_HEADER;
    s->response[length++] = digits[(code >> 4) & 15];
    s->response[length++] = digits[code & 15];
    if (!code && data) {
        for (u32 i = 0; i < 8; ++i) {
            u8 byte = s->snapshot[page * 8 + i];
            s->response[length++] = digits[byte >> 4];
            s->response[length++] = digits[byte & 15];
        }
    }
    u32 crc = custom_transport_crc(s->response, length);
    for (u32 shift = 16; shift; shift -= 4)
        s->response[length++] = digits[(crc >> (shift - 4)) & 15];
    s->response[length] = 0;
    s->response_length = length;
    s->pending = 1;
}

/* Take the complete value once, before constructing any page response. */
static u32 custom_transport_snapshot(volatile CustomTransportState *s, volatile u32 *h,
                                      u32 query)
{
    u32 n = query & 0xffU;
    if ((query >> 8) == 'R') {
        CustomSet set;
        u8 block[CUSTOM_BLOCK_SIZE];
        if (!custom_slot_read(CUSTOM_SLOT_RECORD(CUSTOM_SLOTS_SETTINGS()), n, &set) ||
            !custom_block_encode(&set, block)) return CUSTOM_SLOTS_UNREADABLE;
        for (u32 i = 0; i < CUSTOM_BLOCK_SIZE; ++i) s->snapshot[i] = block[i];
        s->snapshot_length = CUSTOM_BLOCK_SIZE;
    } else {
        volatile u32 *name = h + GRKIT_HOST_CUSTOM_NAMES / 4 + (n - 1) * CUSTOM_SLOTS_NAME_WORDS;
        for (u32 i = 0; i < CUSTOM_SLOTS_NAME_SIZE; ++i)
            s->snapshot[i] = (u8)(name[i / 3] >> (8 * (i % 3)));
        s->snapshot_length = CUSTOM_SLOTS_NAME_SIZE;
    }
    s->query = query;
    s->pages = 1;
    s->mode = CUSTOM_TRANSPORT_READ;
    return 0;
}

/* A staged prefix or complete earlier segment can be retried without writing
   it again. Overlapping new data and gaps are rejected. */
static u32 custom_transport_append(volatile CustomTransportState *s, u32 text,
                                    u32 offset, u32 count, u32 *advanced)
{
    if (!count || offset > s->length || offset + count > 2 + 2 * CUSTOM_BLOCK_SIZE)
        return CUSTOM_SLOTS_BAD_REQUEST;
    if (offset < s->length) {
        if (offset + count > s->length) return CUSTOM_SLOTS_BAD_REQUEST;
        for (u32 i = 0; i < count; ++i)
            if (s->stage[offset + i] != BYTE(text, CUSTOM_TRANSPORT_HEADER + i))
                return CUSTOM_TRANSPORT_CONFLICT;
        return 0;
    }
    for (u32 i = 0; i < count; ++i) s->stage[s->length++] = BYTE(text, CUSTOM_TRANSPORT_HEADER + i);
    s->stage[s->length] = 0;
    *advanced = 1;
    return 0;
}

static u32 custom_transport_dispatch(volatile CustomTransportState *s, volatile u32 *h,
                                     u32 ctx, u32 text, u32 length, u32 id, u32 index,
                                     u32 low, u32 high, u32 *data, u32 *closed)
{
    u8 action = BYTE(text, 13);
    u32 count = length - CUSTOM_TRANSPORT_HEADER, code = CUSTOM_SLOTS_BAD_REQUEST, advanced = 0;
    u32 initiates = action == 'P' || action == 'B' || action == 'Q';
    if (s->id && id != s->id) return initiates ? CUSTOM_TRANSPORT_BUSY : CUSTOM_SLOTS_BAD_REQUEST;
    if (action == 'Z' && !index && !count) {
        custom_transport_clear(s);
        *closed = 1;
        return 0;
    }
    if (!s->id) {
        if (index || !initiates || (action == 'P' && count) ||
            (action == 'B' && (!count || (BYTE(text, 16) != 'W' && BYTE(text, 16) != 'N'))) ||
            (action == 'Q' && (count != 2 || (BYTE(text, 16) != 'R' && BYTE(text, 16) != 'M') ||
                              (BYTE(text, 17) != '1' && BYTE(text, 17) != '2'))))
            return CUSTOM_SLOTS_BAD_REQUEST;
        custom_transport_begin(s, h, id, low, high);
        advanced = 1;
    }
    if (action == 'P' && !index && !count && s->mode == CUSTOM_TRANSPORT_PROBE) code = 0;
    else if (action == 'B' && !index && count &&
             (BYTE(text, 16) == 'W' || BYTE(text, 16) == 'N') &&
             (s->mode == CUSTOM_TRANSPORT_PROBE || s->mode == CUSTOM_TRANSPORT_WRITE)) {
        code = custom_transport_append(s, text, 0, count, &advanced);
        if (!code) s->mode = CUSTOM_TRANSPORT_WRITE;
    } else if (action == 'A' && s->mode == CUSTOM_TRANSPORT_WRITE) {
        code = custom_transport_append(s, text, index, count, &advanced);
    } else if (action == 'X' && (s->mode == CUSTOM_TRANSPORT_WRITE || s->mode == CUSTOM_TRANSPORT_COMMITTED)) {
        u32 crc = 0, valid = count == 4 && custom_transport_number(text, 16, 4, &crc) && index == s->length;
        if (s->mode == CUSTOM_TRANSPORT_COMMITTED) {
            if (valid && crc == s->commit_crc) code = s->commit_code;
        } else {
            s->mode = CUSTOM_TRANSPORT_COMMITTED;
            s->commit_crc = crc;
            if (valid && crc == custom_transport_crc(s->stage, s->length)) {
                u32 source = CUSTOM_TRANSPORT_TEXT_ADDRESS(s->stage, s->length);
                code = custom_slots_run(h, ctx, source, s->length);
                advanced = code == 0;
            }
            s->commit_code = code;
        }
    } else if (action == 'Q' && count == 2 &&
               (BYTE(text, 16) == 'R' || BYTE(text, 16) == 'M') &&
               (BYTE(text, 17) == '1' || BYTE(text, 17) == '2')) {
        u32 query = ((u32)BYTE(text, 16) << 8) | (BYTE(text, 17) - '0');
        u32 pages = BYTE(text, 16) == 'R' ? 7U : 4U;
        if (index < pages) {
            if (s->mode == CUSTOM_TRANSPORT_PROBE && index == 0) {
                code = custom_transport_snapshot(s, h, query);
                advanced = code == 0;
            } else if (s->mode == CUSTOM_TRANSPORT_READ && s->query == query) {
                code = 0;
                if (!(s->pages & (1U << index))) {
                    s->pages |= 1U << index;
                    advanced = 1;
                }
            }
        }
        *data = code == 0;
    }
    if (!code && advanced) {
        s->tick_low = low;
        s->tick_high = high;
    }
    return code;
}

/* Recognized protocol errors use a CRC-protected response. Invalid framing
   is refused at the property layer. A native writer retains buffer ownership. */
static u32 custom_transport_set(volatile u32 *h, u32 ctx, u32 command)
{
    volatile CustomTransportState *s = &CUSTOM_TRANSPORT_STATE;
    u32 low, high;
    CUSTOM_TRANSPORT_NOW(low, high);
    custom_transport_sync(s, h, low, high);
    if (!CUSTOM_SLOTS_RUNNING(h)) return 0;
    u32 property = WORD(command, CUSTOM_SLOTS_COMMAND_VALUE_AT);
    u32 bytes = WORD(command, CUSTOM_SLOTS_COMMAND_SIZE_AT);
    if (!property || bytes < 4 || WORD(property, 0) != CUSTOM_TRANSPORT_PROPERTY) return 0;
    if (bytes < CUSTOM_SLOTS_TEXT_AT || WORD(property, 4) != CUSTOM_SLOTS_STRING || WORD(property, 8) != 1) {
        custom_transport_clear(s);
        return 0;
    }
    u32 size = WORD(property, 12), text = property + CUSTOM_SLOTS_TEXT_AT;
    static const u8 prefix[] = {0x1b, 'G', 'R', 'K', '1'};
    u32 recognized = size >= 5 && bytes >= CUSTOM_SLOTS_TEXT_AT + 5;
    for (u32 i = 0; recognized && i < 5; ++i) recognized = BYTE(text, i) == prefix[i];
    if (!recognized) {
        custom_transport_clear(s);
        return 0;
    }
    u32 length = 0, id = 0, index = 0, data = 0, closed = 0;
    if (size <= 64 && size <= bytes - CUSTOM_SLOTS_TEXT_AT) {
        while (length < size && BYTE(text, length)) ++length;
        if (length < size && length >= CUSTOM_TRANSPORT_HEADER && length <= CUSTOM_TRANSPORT_TEXT_MAX &&
            custom_transport_number(text, 5, 8, &id) && id && custom_transport_number(text, 14, 2, &index)) {
            u32 code = custom_transport_dispatch(s, h, ctx, text, length, id, index, low, high, &data, &closed);
            if (!closed) {
                /* Invalid initial requests have a reply lease, but do not
                   reserve an ID or prevent a valid transaction from starting. */
                if (!s->id && !s->pending) {
                    s->epoch = CUSTOM_TRANSPORT_EPOCH(h);
                    s->tick_low = low;
                    s->tick_high = high;
                }
                custom_transport_reply(s, text, code, index, data);
            }
            CUSTOM_SLOTS_RELEASE(property);
            custom_transport_result(command, 0, 0, 0, 0);
            return 2;
        }
    }
    CUSTOM_SLOTS_RELEASE(property);
    custom_transport_result(command, 1, CUSTOM_SLOTS_BAD_REQUEST, 0, 0);
    return 1;
}

/* bled re-fetches this property for every Read Blob fragment. Preserve the
   snapshot and reply across GETs, but never renew their lease. A client must
   check the response CRC because expiration can occur between fragments. */
static u32 custom_transport_get(volatile u32 *h, u32 ctx, u32 command)
{
    (void)ctx;
    volatile CustomTransportState *s = &CUSTOM_TRANSPORT_STATE;
    u32 low, high;
    CUSTOM_TRANSPORT_NOW(low, high);
    custom_transport_sync(s, h, low, high);
    if (!CUSTOM_SLOTS_RUNNING(h) || WORD(command, 12) != CUSTOM_TRANSPORT_PROPERTY || !s->pending) return 0;
    u32 size = (s->response_length + 4) & ~3U;
    u32 bytes = 16 + size, property = CUSTOM_TRANSPORT_ALLOC(bytes);
    if (!property) {
        custom_transport_result(command, 1, 4, 0, 0);
        return 1;
    }
    for (u32 i = 0; i < bytes; ++i) BYTE(property, i) = 0;
    WORD(property, 0) = CUSTOM_TRANSPORT_PROPERTY;
    WORD(property, 4) = CUSTOM_SLOTS_STRING;
    WORD(property, 8) = 1;
    WORD(property, 12) = size;
    for (u32 i = 0; i < s->response_length; ++i) BYTE(property, 16 + i) = s->response[i];
    custom_transport_result(command, 0, 0, bytes, property);
    return 2;
}
#endif
