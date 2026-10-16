#include <assert.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned int u32;
#include "catalog.h"

#define CAPACITY 3U
#define MARK 100U
static u8 profiles[CAPACITY][GRKIT_GRF_SIZE];
static char names[CAPACITY][CATALOG_NAME_SIZE];

struct candidate { const char *id, *name; u8 mark; };
/* "beta" appears twice, so its smallest file name must win. "epsilon" and
   "gamma" exceed the capacity after the three smallest IDs. */
static const struct candidate candidates[] = {
    {"epsilon", "E.GRF", 1}, {"beta", "B2.GRF", 2}, {"alpha", "A.GRF", 3},
    {"delta", "D.GRF", 4}, {"beta", "B1.GRF", 5}, {"gamma", "G.GRF", 6},
};
#define CANDIDATES (sizeof(candidates) / sizeof(candidates[0]))

static u32 offer(u32 count, const struct candidate *c)
{
    static u8 record[GRKIT_GRF_SIZE];
    memset(record, 0, sizeof(record));
    strcpy((char *)record + CATALOG_ID, c->id);
    record[MARK] = c->mark;
    return catalog_offer(profiles, names, count, CAPACITY, record, c->name);
}
static void expect(u32 count, const char *const *ids, const u8 *marks, u32 n)
{
    assert(count == n);
    for (u32 i = 0; i < n; ++i) {
        assert(!strcmp((const char *)profiles[i] + CATALOG_ID, ids[i]));
        assert(profiles[i][MARK] == marks[i]);
    }
}
static void names_follow_the_effect_file_rule(void)
{
    static const char *const accepted[] = {"A.GRF", "mono.grf", "ABCDEFGH.GRF", "NEG_1.Grf"};
    static const char *const rejected[] = {
        "._MONO.GRF", ".GRF", ".MONO.GRF", "ABCDEFGHI.GRF", "MONO-1.GRF",
        "MONO.TXT", "MONO.GRFX", "MONO GRF", "",
    };
    for (u32 i = 0; i < sizeof(accepted) / sizeof(accepted[0]); ++i)
        assert(catalog_name(accepted[i], strlen(accepted[i])) == 1);
    for (u32 i = 0; i < sizeof(rejected) / sizeof(rejected[0]); ++i)
        assert(catalog_name(rejected[i], strlen(rejected[i])) == 0);
}
static void duplicate_ids_keep_the_smallest_file_name(void)
{
    static const char *const ids[] = {"beta"};
    static const u8 b1[] = {5};
    u32 count = offer(0, &candidates[1]);
    count = offer(count, &candidates[4]);
    expect(count, ids, b1, 1);
    count = offer(0, &candidates[4]);
    count = offer(count, &candidates[1]);
    expect(count, ids, b1, 1);
}
/* Heap's algorithm visits every directory order of the candidates. */
static void selection_is_independent_of_directory_order(void)
{
    static const char *const ids[] = {"alpha", "beta", "delta"};
    static const u8 marks[] = {3, 5, 4};
    u32 order[CANDIDATES], stack[CANDIDATES] = {0}, permutations = 0;
    for (u32 i = 0; i < CANDIDATES; ++i) order[i] = i;
    for (u32 i = 0;;) {
        u32 count = 0;
        for (u32 j = 0; j < CANDIDATES; ++j) count = offer(count, &candidates[order[j]]);
        expect(count, ids, marks, CAPACITY);
        ++permutations;
        while (i < CANDIDATES && stack[i] >= i) stack[i++] = 0;
        if (i == CANDIDATES) break;
        u32 other = i & 1 ? stack[i] : 0, swap = order[other];
        order[other] = order[i]; order[i] = swap;
        ++stack[i]; i = 0;
    }
    assert(permutations == 720);
}
int main(void)
{
    names_follow_the_effect_file_rule();
    duplicate_ids_keep_the_smallest_file_name();
    selection_is_independent_of_directory_order();
    return 0;
}
