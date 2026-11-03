#include <assert.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
#include "relocation.h"

/* A small RTOS image: startup code and literals, code, then the .data copy
   source. The second image moves the code and the data, as another build
   of the same version does. */
#define BASE 0x53000000U
#define SIZE 0x40000U
#define DATA_SIZE 0x100U
#define BSS_END 0x55001000U
static u32 image[SIZE / 4];
static u32 read_word(u32 address)
{
    if (address < BASE || address - BASE >= SIZE) return 0xdeadbeefU;
    return image[(address - BASE) / 4];
}
static void put(u32 address, u32 value) { image[(address - BASE) / 4] = value; }

static void startup(u32 data_source)
{
    for (u32 a = 0x530003f0U; a < RELOCATION_STARTUP; a += 4) put(a, 0xe1a00000U + a);
    put(RELOCATION_STARTUP, data_source);
    put(RELOCATION_STARTUP + 4, RELOCATION_DATA_START);
    put(RELOCATION_STARTUP + 8, DATA_SIZE);
    put(RELOCATION_STARTUP + 12, RELOCATION_DATA_START + DATA_SIZE);
    put(RELOCATION_STARTUP + 16, BSS_END);
}

/* A function at `at`: a prologue, a BL to `callee`, a literal that holds the
   BSS address `variable`, a MOVW and MOVT pair of it, and distinct words. */
static void function(u32 at, u32 callee, u32 variable, u32 seed)
{
    put(at, 0xe1a0c00dU);
    put(at + 4, 0xe92dd800U);
    put(at + 8, 0xeb000000U | (((callee - at - 16) >> 2) & 0xffffffU));
    put(at + 12, 0xe5900000U + seed);
    put(at + 16, 0xe3000000U | ((variable & 0xf000U) << 4) | (variable & 0xfffU));
    put(at + 20, 0xe3400000U | ((variable >> 12) & 0xf0000U) | ((variable >> 16) & 0xfffU));
    put(at + 24, 0xe89da800U);
    put(at + 28, variable);
}

/* Window 0: the startup code. Window 1: the function. Window 2: .data
   content. Window 3: the callee, compared at the branch target only. */
static u32 words[64];
static struct relocation_window windows[4];
static struct relocation_source sources[16];
static u32 symbols[8];
static struct relocation_tables tables;
static void pinned(void)
{
    u32 n = 0;
    memset(image, 0, sizeof(image));
    startup(0x53020000U);
    function(0x53010000U, 0x53010100U, 0x55000800U, 7);
    put(0x53010100U, 0xe3a00001U); put(0x53010104U, 0xe12fff1eU);
    for (u32 i = 0; i < 8; ++i) put(0x53020000U + 0x40 + i * 4, 0x1000U + i * 3);
    windows[0] = (struct relocation_window){0x530003f0U, n, 24, 0, REGION_IMAGE, 0};
    for (u32 i = 0; i < 24; ++i) words[n++] = relocation_mask(read_word(0x530003f0U + i * 4));
    windows[1] = (struct relocation_window){0x53010000U, n, 8, 3, REGION_IMAGE, 1};
    for (u32 i = 0; i < 8; ++i) words[n++] = relocation_mask(read_word(0x53010000U + i * 4));
    windows[2] = (struct relocation_window){0x55000040U, n, 8, 0, REGION_DATA, 1};
    for (u32 i = 0; i < 8; ++i) words[n++] = relocation_mask(read_word(0x53020040U + i * 4));
    windows[3] = (struct relocation_window){0x53010100U, n, 2, 0, REGION_IMAGE, 0};
    for (u32 i = 0; i < 2; ++i) words[n++] = relocation_mask(read_word(0x53010100U + i * 4));
    /* Symbols: the function +8, the callee, a .data word, the BSS variable,
       the BSS end, and the tail. */
    u32 s = 0;
    symbols[0] = 0x53010008U; sources[s++] = (struct relocation_source){0, SOURCE_WINDOW, 0, 1, RELOCATION_NONE, 8};
    symbols[1] = 0x53010104U; sources[s++] = (struct relocation_source){1, SOURCE_BRANCH, 0, 1, 3, 2};
    symbols[2] = 0x55000048U; sources[s++] = (struct relocation_source){2, SOURCE_WINDOW, 0, 2, RELOCATION_NONE, 8};
    symbols[3] = 0x55000800U;
    sources[s++] = (struct relocation_source){3, SOURCE_LITERAL, 0, 1, RELOCATION_NONE, 7};
    sources[s++] = (struct relocation_source){3, SOURCE_PAIR, 0, 1, 5, 4};
    symbols[4] = BSS_END; sources[s++] = (struct relocation_source){4, SOURCE_STARTUP, 0, 0, RELOCATION_NONE, 4};
    symbols[5] = 0x53021000U; sources[s++] = (struct relocation_source){5, SOURCE_TAIL, 0, 0, RELOCATION_NONE, 0};
    /* The callee as a hook target: the branch alone. */
    symbols[6] = 0x53010100U; sources[s++] = (struct relocation_source){6, SOURCE_BRANCH, 0, 1, RELOCATION_NONE, 2};
    tables = (struct relocation_tables){windows, 4, words, symbols, 7, sources, s, 0, 0x10000U, 0x9000U};
}

static u32 window_at[4], symbol_at[8];
static struct relocation_state state;
static const u32 *hint;
static int resolve(void)
{
    memset(&state, 0, sizeof(state));
    state.read = read_word; state.window_at = window_at; state.symbol_at = symbol_at; state.hint = hint;
    return relocation_resolve(&tables, &state);
}

int main(void)
{
    /* The masks clear branch offsets, RTOS addresses, and MOVW immediates. */
    assert(relocation_mask(0xeb001234U) == 0xeb000000U);
    assert(relocation_mask(0x5437b640U) == RELOCATION_ADDRESS_MASK);
    assert(relocation_mask(0xe3051610U) == 0xe3001000U);
    assert(relocation_mask(0xe1a0c00dU) == 0xe1a0c00dU);

    /* The pinned image resolves to itself, without a search. */
    pinned();
    assert(resolve());
    assert(!state.searched);
    assert(state.image_end == 0x53020100U && state.tail == 0x53021000U);
    for (u32 i = 0; i < 7; ++i) assert(symbol_at[i] == symbols[i]);

    /* Another build: the function, the callee, and the .data content move,
       and the variable moves in the BSS. */
    memset(image, 0, sizeof(image));
    startup(0x53024000U);
    function(0x53010300U, 0x53010500U, 0x55000900U, 7);
    put(0x53010500U, 0xe3a00001U); put(0x53010504U, 0xe12fff1eU);
    for (u32 i = 0; i < 8; ++i) put(0x53024000U + 0x60 + i * 4, 0x1000U + i * 3);
    assert(resolve());
    assert(symbol_at[0] == 0x53010308U);
    assert(symbol_at[1] == 0x53010504U);
    assert(symbol_at[2] == 0x55000068U);
    assert(symbol_at[3] == 0x55000900U);
    assert(symbol_at[4] == BSS_END);
    assert(state.image_end == 0x53024100U && symbol_at[5] == 0x53025000U);
    assert(symbol_at[6] == 0x53010500U);
    assert(relocation_delta(&tables, &state, 0) == 0x300U);
    assert(state.searched);

    /* The addresses of that search as a hint: the resolve checks the windows
       there and does not search. A wrong hint falls back to the search. */
    u32 found[4];
    memcpy(found, window_at, sizeof(found));
    hint = found;
    assert(resolve() && !state.searched && symbol_at[0] == 0x53010308U);
    found[1] += 4;
    assert(resolve() && state.searched && symbol_at[0] == 0x53010308U);
    hint = 0;

    /* A changed instruction in the function: the window is not found. */
    put(0x5301030cU, 0xe5900008U);
    assert(!resolve());
    assert(state.failure[0].reason == RELOCATION_NOT_FOUND && state.failure[0].index == 1);
    put(0x5301030cU, 0xe5900007U);

    /* A second copy of the function within the radius: ambiguous. */
    function(0x53018000U, 0x53010500U, 0x55000900U, 7);
    assert(!resolve());
    assert(state.failure[0].reason == RELOCATION_AMBIGUOUS && state.failure[0].count == 2);
    for (u32 a = 0x53018000U; a < 0x53018020U; a += 4) put(a, 0);

    /* The callee differs: the verify window fails. */
    put(0x53010500U, 0xe3a00002U);
    assert(!resolve());
    assert(state.failure[0].reason == RELOCATION_VERIFY);
    put(0x53010500U, 0xe3a00001U);

    /* The literal and the pair name different variables: they disagree. */
    put(0x5301031cU, 0x55000904U);
    assert(!resolve());
    assert(state.failure[0].reason == RELOCATION_DISAGREE && state.failure[0].index == 3);
    put(0x5301031cU, 0x55000900U);

    /* A branch target after the image lies where the private code goes. The
       verify window keeps the callee symbol, so only the hook target fails. */
    put(0x53010308U, 0xeb000000U | (((0x53025000U - 0x53010308U - 8) >> 2) & 0xffffffU));
    put(0x53025000U, 0xe3a00001U); put(0x53025004U, 0xe12fff1eU);
    assert(!resolve());
    int outside = 0;
    for (u32 i = 0; i < state.failures; ++i)
        outside |= state.failure[i].reason == RELOCATION_OUTSIDE && state.failure[i].index == 6;
    assert(outside);
    put(0x53010308U, 0xeb000000U | (((0x53010500U - 0x53010308U - 8) >> 2) & 0xffffffU));
    assert(resolve());

    /* A MOVW and MOVT pair of 0 is a failure, not an address. */
    put(0x53010310U, 0xe3000000U); put(0x53010314U, 0xe3400000U);
    assert(!resolve());
    put(0x53010310U, 0xe3000900U); put(0x53010314U, 0xe3450500U);
    assert(resolve());

    /* Invalid startup literals and a tail without room stop the resolver. */
    put(RELOCATION_STARTUP + 4, 0x55001000U);
    assert(!resolve() && state.failure[0].reason == RELOCATION_STARTUP_INVALID);
    put(RELOCATION_STARTUP + 4, RELOCATION_DATA_START);
    put(0x530003f4U, 0);
    assert(!resolve() && state.failure[0].reason == RELOCATION_STARTUP_INVALID);
    put(0x530003f4U, 0xe1a00000U + 0x530003f4U);
    put(RELOCATION_STARTUP, 0x54ff8000U); put(RELOCATION_STARTUP + 8, 0x1000U);
    assert(!resolve() && state.failure[0].reason == RELOCATION_STARTUP_INVALID);
    put(RELOCATION_STARTUP, 0x53024000U); put(RELOCATION_STARTUP + 8, DATA_SIZE);
    assert(resolve());

    /* The actual core span can grow past the former fixed reservations.
       Check the free RX space of the running image, not the pinned base. */
    startup(0x54fdf000U);
    tables.tail_size = 0x20000U;
    assert(relocation_startup(&tables, &state));
    assert(state.tail == 0x54fe0000U);
    tables.tail_size += 4;
    assert(!relocation_startup(&tables, &state));
    tables.tail_size = 0xffffffffU;
    assert(!relocation_startup(&tables, &state));
    /* A different build leaves one less page for the same core. */
    tables.tail_size = 0x20000U;
    startup(0x54fe0000U);
    assert(!relocation_startup(&tables, &state));

    /* Moving references: a word adds the delta; a branch adds the delta of
       its target minus that of its site, within the ARM branch range. */
    u8 word[4] = {0x08, 0x00, 0x01, 0x53};
    assert(relocation_move(word, 0, 0x300U, 0));
    assert(word[0] == 0x08 && word[1] == 0x03 && word[2] == 0x01 && word[3] == 0x53);
    u8 branch[4] = {0x10, 0x00, 0x00, 0xeb};
    assert(relocation_move(branch, 1, 0x2000U, 0x1000U));
    assert(branch[0] == 0x10 && branch[1] == 0x04 && branch[2] == 0x00 && branch[3] == 0xeb);
    assert(relocation_move(branch, 1, 0, 0x1000U));
    assert(branch[0] == 0x10 && branch[1] == 0x00 && branch[2] == 0x00 && branch[3] == 0xeb);
    u8 far[4] = {0xff, 0xff, 0x7f, 0xea};
    assert(!relocation_move(far, 1, 4, 0));
    u8 other[4] = {0x00, 0x00, 0xa0, 0xe1};
    assert(!relocation_move(other, 1, 4, 0));
    return 0;
}
