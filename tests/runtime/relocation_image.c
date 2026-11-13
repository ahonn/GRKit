/* Resolves the relocation tables of a generated installer header against an
   RTOS image file, then prints "ok" or "failed", after a failure the points
   that it drops, each failure, and each symbol as "symbol <pinned>
   <resolved>". */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
struct patch { u32 address, size, kind; const u8 *before; u8 *after; };
struct guard { u32 address, size; const u8 *bytes; };
#include "relocation.h"
#include "module_payload.h"

static u8 *image;
static long image_size;
static u32 read_word(u32 address)
{
    u32 value = 0xffffffffU;
    if (address >= RELOCATION_RTOS_BASE && address - RELOCATION_RTOS_BASE + 4 <= (u32)image_size)
        memcpy(&value, image + (address - RELOCATION_RTOS_BASE), 4);
    return value;
}

int main(int argc, char **argv)
{
    FILE *file = argc == 2 ? fopen(argv[1], "rb") : NULL;
    if (!file || fseek(file, 0, SEEK_END) || (image_size = ftell(file)) <= 0) return 2;
    rewind(file);
    image = malloc((size_t)image_size);
    if (!image || fread(image, 1, (size_t)image_size, file) != (size_t)image_size) return 2;
    fclose(file);
    static u32 window_at[RELOCATION_WINDOW_COUNT];
    static struct relocation_state state;
    state.read = read_word;
    state.window_at = window_at;
    state.symbol_at = relocated;
    int ok = relocation_resolve(&relocation_tables, &state);
    printf("%s %#x\n", ok ? "ok" : "failed", state.tail);
    /* The extension points that a failed resolve drops, or "needed" when a
       symbol that each install needs did not resolve (installer.c). */
    u32 dropped;
    if (!ok) {
        if (relocation_dropped(&relocation_tables, &state, symbol_points, &dropped)) printf("dropped %#x\n", dropped);
        else printf("dropped needed\n");
    }
    for (u32 i = 0; i < state.failures && i < RELOCATION_REPORT; ++i)
        printf("failure %u %u %#x %#x\n", state.failure[i].reason, state.failure[i].index,
               state.failure[i].address, state.failure[i].count);
    for (u32 i = 0; i < RELOCATION_SYMBOL_COUNT; ++i)
        printf("symbol %#x %#x\n", relocation_symbols[i], relocated[i]);
    return 0;
}
