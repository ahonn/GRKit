/* Prints the selective color weight table of picture.c for one parameter set, so
   the Rust test can compare it with grkit_core::selective. argv[1] is the 14
   extension bytes in hex, as a GRF stores them at offset 106. */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define ARENA 0x59000000U
static uint8_t arena[0x10000];
static void *mapped(uint32_t address, uint32_t bytes)
{
    assert(address >= ARENA && address - ARENA + bytes <= sizeof(arena));
    assert(address % bytes == 0);
    return arena + (address - ARENA);
}
#define WORD(p,o) (*(volatile uint32_t *)mapped((p)+(o),4))
#define HALF(p,o) (*(volatile uint16_t *)mapped((p)+(o),2))
#define BYTE(p,o) (*(volatile uint8_t *)mapped((p)+(o),1))
#include "picture.c"

static volatile u32 header[64];
volatile u32 *framing_header(void) { return header; }

int main(int argc, char **argv)
{
    static u8 table[SELECTIVE_TABLE];
    static SelectiveFactors factors;
    const uint32_t record = ARENA;
    assert(argc == 2 && strlen(argv[1]) == 28);
    for (u32 i = 0; i < 14; ++i) {
        unsigned value;
        assert(sscanf(argv[1] + i * 2, "%2x", &value) == 1);
        BYTE(record, GRKIT_RECORD_SELECTIVE + i) = (u8)value;
    }
    selective_table(record, table, &factors);
    fwrite(table, 1, sizeof table, stdout);
    return 0;
}
