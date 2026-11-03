#include <assert.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned int u32;
#include "state.h"

static struct state state;

static void parse(const char *text)
{
    state_parse(&state, (const u8 *)text, (long)strlen(text));
}

static u32 value_of(u32 key)
{
    u32 value = 0xdeadbeefU;
    assert(state_value(&state, key, &value));
    return value;
}

static int has(u32 key)
{
    u32 value;
    return state_value(&state, key, &value);
}

int main(void)
{
    /* CR LF and LF lines, and a last line without a newline. */
    parse("12\r\n3\r\n45\n6\n789\r\n0");
    assert(state.pairs == 3);
    assert(value_of(12) == 3 && value_of(45) == 6 && value_of(789) == 0);
    assert(!has(3));

    /* A missing or invalid value keeps the default; a saved zero overrides it. */
    assert(state_setting_value(&state, 789, 1, 3) == 0);
    assert(state_setting_value(&state, 12, 1, 3) == 1);
    assert(state_setting_value(&state, 999, 2, 3) == 2);
    assert(state_setting_value(&state, 45, 1, 7) == 6);
    assert(state_setting_value(&state, 0, 1, 3) == 1);

    /* Empty lines do not shift the pairs. */
    parse("\r\n\r\n12\r\n\r\n3\r\n\r\n");
    assert(state.pairs == 1 && value_of(12) == 3);

    /* A later pair of a key wins. */
    parse("12\r\n3\r\n12\r\n4\r\n");
    assert(value_of(12) == 4);

    /* A pair with an invalid line is skipped, and the next pair still reads. */
    parse("12\r\nx\r\n13\r\n5\r\n");
    assert(!has(12) && value_of(13) == 5);
    parse("1a\r\n7\r\n13\r\n5\r\n");
    assert(state.pairs == 1 && value_of(13) == 5);
    parse("-1\r\n7\r\n");
    assert(state.pairs == 0);

    /* Nine digits read, ten are invalid. */
    parse("999999999\r\n999999999\r\n1000000000\r\n1\r\n");
    assert(state.pairs == 1 && value_of(999999999U) == 999999999U);

    /* A key without its value is no pair. */
    parse("12\r\n3\r\n14");
    assert(state.pairs == 1 && !has(14));

    /* A missing file gives no pairs, and an empty one too. */
    state_parse(&state, (const u8 *)"", -2);
    assert(state.pairs == 0);
    parse("");
    assert(state.pairs == 0);

    /* The pairs stop at STATE_PAIRS. */
    {
        static char text[STATE_PAIRS * 2 * 8 + 64];
        u32 used = 0;
        for (u32 i = 0; i <= STATE_PAIRS; ++i) {
            char line[16];
            u32 n = 0, key = 1000 + i;
            do { line[n++] = (char)('0' + key % 10); key /= 10; } while (key);
            while (n) text[used++] = line[--n];
            memcpy(text + used, "\r\n1\r\n", 5);
            used += 5;
        }
        text[used] = 0;
        parse(text);
        assert(state.pairs == STATE_PAIRS);
        assert(has(1000 + STATE_PAIRS - 1) && !has(1000 + STATE_PAIRS));
    }

    /* A GRKit 1.2 file: the known keys take the current keys, the Fn
       adjustment entries stay, and other keys go. A later pair still wins. */
    {
        static const u32 from[] = {495043802U, 161811731U}, to[] = {11U, 22U};
        parse("495043802\r\n32769\r\n161811731\r\n2\r\n777\r\n1\r\n"
              "990000005\r\n40\r\n991000000\r\n3\r\n161811731\r\n4\r\n");
        state_translate(&state, from, to, 2, 990000000U);
        assert(state.pairs == 4);
        assert(value_of(11) == 32769 && value_of(22) == 4 && value_of(990000005U) == 40);
        assert(!has(495043802U) && !has(161811731U) && !has(777) && !has(991000000U));
        state_parse(&state, (const u8 *)"", -2);
        state_translate(&state, from, to, 2, 990000000U);
        assert(state.pairs == 0);
    }
    return 0;
}
