/* Finds the firmware addresses of the private code in the running build of the
   firmware version (crates/grkit-build/src/relocation.rs gives the method).

   The build gives windows of masked words from the pinned image, and for each
   symbol one or more sources: a window offset, the target of a branch in a
   window, a literal or a MOVW and MOVT pair in a window, a startup literal, or
   the private tail. The resolver finds each searched window once within the
   radius of its pinned start, then computes each symbol from its sources. All
   sources of a symbol must agree.

   The includer defines u8, u16, and u32. */
#ifndef GRKIT_RELOCATION_H
#define GRKIT_RELOCATION_H

#define RELOCATION_RTOS_BASE 0x53000000U
#define RELOCATION_DATA_START 0x55000000U
#define RELOCATION_DATA_LIMIT 0x60000000U
#define RELOCATION_ADDRESS_MASK 0x5a5a5a5aU
/* Source, destination, and size of the .data copy, then the BSS start and end. */
#define RELOCATION_STARTUP 0x53000450U

enum { REGION_IMAGE, REGION_DATA };
enum { SOURCE_WINDOW, SOURCE_BRANCH, SOURCE_LITERAL, SOURCE_PAIR, SOURCE_STARTUP, SOURCE_TAIL };
/* Failure reasons in a report entry. */
enum {
    RELOCATION_STARTUP_INVALID = 1, /* startup literals or code */
    RELOCATION_NOT_FOUND,           /* a window: no match */
    RELOCATION_AMBIGUOUS,           /* a window: more than one match */
    RELOCATION_NO_BRANCH,           /* a source: no B or BL at the word */
    RELOCATION_VERIFY,              /* a source: the verify window differs */
    RELOCATION_OUTSIDE,             /* a symbol outside its region */
    RELOCATION_DISAGREE,            /* the sources of a symbol differ */
};
#define RELOCATION_NONE 0xffffU

/* A window: its pinned start, its first word in the word pool, its length,
   the word that the search compares first (the rarest one near the start),
   its region, and whether the resolver searches it. */
struct relocation_window { u32 start, first; u16 count, probe; u8 region, search; };
/* WINDOW: a = byte offset. BRANCH: a = word index, b = verify window or
   RELOCATION_NONE. LITERAL: a = word index. PAIR: a = MOVW index, b = MOVT
   index. STARTUP: a = literal index. TAIL: none. */
struct relocation_source { u16 symbol; u8 kind, pad; u16 window, b; u32 a; };
struct relocation_tables {
    const struct relocation_window *windows;
    u32 window_count;
    const u32 *words;
    const u32 *symbols;
    u32 symbol_count;
    const struct relocation_source *sources;
    u32 source_count;
    u32 startup_window, radius, tail_size;
};
struct relocation_failure { u8 reason, pad; u16 index; u32 address, count; };
#define RELOCATION_REPORT 8U
struct relocation_state {
    u32 (*read)(u32 address);
    /* The startup literals of the running image. */
    u32 data_source, data_size, bss_start, bss_end, image_end, tail;
    u32 *window_at, *symbol_at;
    /* The window addresses that an earlier search of the same build found,
       or 0. After a resolve, `searched` tells whether the search ran. */
    const u32 *hint;
    /* Optional: the windows that only optional symbols use (nonzero), which
       a hint may give as not found, or 0 when each window is needed. */
    const u32 *optional;
    /* Read-only discovery can request a symbol subset after unrelated hook
       windows have changed. NULL retains full installation validation. */
    const u8 *required;
    u32 searched;
    u32 failures;
    struct relocation_failure failure[RELOCATION_REPORT];
};

static u32 relocation_mask(u32 word)
{
    if (((word >> 25) & 7) == 5) return word & 0xff000000U;
    if (word - 0x53000000U < 0x03000000U) return RELOCATION_ADDRESS_MASK;
    if ((word & 0x0ff00000U) == 0x03000000U || (word & 0x0ff00000U) == 0x03400000U)
        return word & 0xfff0f000U;
    return word;
}

static void relocation_fail(struct relocation_state *s, u32 reason, u32 index, u32 address, u32 count)
{
    if (s->failures < RELOCATION_REPORT) {
        struct relocation_failure *f = &s->failure[s->failures];
        f->reason = (u8)reason; f->pad = 0; f->index = (u16)index; f->address = address; f->count = count;
    }
    ++s->failures;
}

/* The RTOS address that holds the word at `address` of a region. */
static u32 relocation_source_of(const struct relocation_state *s, u32 region, u32 address)
{
    return region == REGION_DATA ? address - RELOCATION_DATA_START + s->data_source : address;
}

/* The region limits in the running image, in region addresses. */
static void relocation_limits(const struct relocation_state *s, u32 region, u32 *low, u32 *high)
{
    if (region == REGION_DATA) { *low = RELOCATION_DATA_START; *high = RELOCATION_DATA_START + s->data_size; }
    else { *low = RELOCATION_RTOS_BASE; *high = s->data_source; }
}

/* Whether window `w` lies at `at` (a region address) in the running image. */
static int relocation_matches(const struct relocation_tables *t, struct relocation_state *s, u32 w, u32 at)
{
    const struct relocation_window *window = &t->windows[w];
    u32 low, high;
    relocation_limits(s, window->region, &low, &high);
    if (at < low || at > high || (high - at) / 4 < window->count || (at & 3)) return 0;
    u32 source = relocation_source_of(s, window->region, at);
    for (u32 i = 0; i < window->count; ++i)
        if (relocation_mask(s->read(source + i * 4)) != t->words[window->first + i]) return 0;
    return 1;
}

/* The arithmetic delta of a symbol: new address minus pinned address. */
static u32 relocation_delta(const struct relocation_tables *t, const struct relocation_state *s, u32 symbol)
{
    return s->symbol_at[symbol] - t->symbols[symbol];
}

static int relocation_startup(const struct relocation_tables *t, struct relocation_state *s)
{
    u32 source = s->read(RELOCATION_STARTUP), start = s->read(RELOCATION_STARTUP + 4);
    s->data_source = source; s->data_size = s->read(RELOCATION_STARTUP + 8);
    s->bss_start = s->read(RELOCATION_STARTUP + 12); s->bss_end = s->read(RELOCATION_STARTUP + 16);
    s->image_end = source + s->data_size;
    if (start != RELOCATION_DATA_START || (source & 3) || (s->data_size & 3) || !s->data_size ||
        source < RELOCATION_RTOS_BASE || source >= RELOCATION_DATA_START ||
        RELOCATION_DATA_START - source < s->data_size ||
        s->bss_start - RELOCATION_DATA_START < s->data_size || s->bss_end <= s->bss_start ||
        s->bss_end >= RELOCATION_DATA_LIMIT) {
        relocation_fail(s, RELOCATION_STARTUP_INVALID, 0, RELOCATION_STARTUP, 0);
        return 0;
    }
    /* The first 4 KiB boundary after the image, which must leave room for the
       private code below the end of the executable mapping. */
    s->tail = (s->image_end + 0xfffU) & ~0xfffU;
    if (s->tail < s->image_end || RELOCATION_DATA_START - s->tail < t->tail_size ||
        !relocation_matches(t, s, t->startup_window, t->windows[t->startup_window].start)) {
        relocation_fail(s, RELOCATION_STARTUP_INVALID, t->startup_window, RELOCATION_STARTUP, 1);
        return 0;
    }
    return 1;
}

static int relocation_window_required(const struct relocation_tables *t,const struct relocation_state *s,u32 window)
{
    if(!s->required) return 1;
    for(u32 i=0;i<t->source_count;++i) {
        const struct relocation_source *source=&t->sources[i];
        if(s->required[source->symbol] && source->kind!=SOURCE_STARTUP && source->kind!=SOURCE_TAIL &&
           source->window==window) return 1;
    }
    return 0;
}

/* Whether each searched window lies at the address that `at` gives it: its
   pinned start, or the address that an earlier search of the same build
   found (`hint`). A camera reads the image uncached, so the search of a
   moved build takes seconds, and a check at known addresses milliseconds.
   The build checked that each window is unique in the pinned image, and
   the search checked it in the build of the hint. */
static int relocation_known(const struct relocation_tables *t, struct relocation_state *s, const u32 *at)
{
    for (u32 w = 0; w < t->window_count; ++w) {
        const struct relocation_window *window = &t->windows[w];
        s->window_at[w] = 0;
        if (!window->search || !relocation_window_required(t,s,w)) continue;
        u32 address = at ? at[w] : window->start;
        /* A search of this build did not find an optional window. Its
           symbols do not resolve, as after that search. */
        if (!address && at && s->optional && s->optional[w]) {
            relocation_fail(s, RELOCATION_NOT_FOUND, w, window->start, 0);
            continue;
        }
        if (!address || !relocation_matches(t, s, w, address)) return 0;
        s->window_at[w] = address;
    }
    return 1;
}

/* Finds each searched window once within the radius of its pinned start. */
static void relocation_search(const struct relocation_tables *t, struct relocation_state *s)
{
    s->searched = 0;
    if (relocation_known(t, s, 0) || (s->hint && relocation_known(t, s, s->hint))) return;
    /* The search reports again what a check of the hint reported. */
    s->failures = 0;
    s->searched = 1;
    for (u32 w = 0; w < t->window_count; ++w) {
        const struct relocation_window *window = &t->windows[w];
        s->window_at[w] = 0;
        if (!window->search || !relocation_window_required(t,s,w)) continue;
        u32 low, high, matches = 0, first = 0;
        relocation_limits(s, window->region, &low, &high);
        u32 from = window->start - low > t->radius ? window->start - t->radius : low;
        u32 to = high - window->start > t->radius ? window->start + t->radius : high;
        u32 probe = t->words[window->first + window->probe], offset = window->probe * 4U;
        for (u32 at = from; at <= to && matches < 2; at += 4) {
            if (at >= high || high - at <= offset) break;
            if (relocation_mask(s->read(relocation_source_of(s, window->region, at + offset))) != probe) continue;
            if (!relocation_matches(t, s, w, at)) continue;
            if (!matches) first = at;
            ++matches;
        }
        if (matches == 1) s->window_at[w] = first;
        else relocation_fail(s, matches ? RELOCATION_AMBIGUOUS : RELOCATION_NOT_FOUND, w, window->start, matches);
    }
}

/* The value that source `i` gives, or 0 after a failure. A window that was
   not found is already in the report. */
static u32 relocation_read_source(const struct relocation_tables *t, struct relocation_state *s, u32 i)
{
    const struct relocation_source *source = &t->sources[i];
    u32 symbol = t->symbols[source->symbol], at = 0, word;
    if (source->kind == SOURCE_STARTUP) return s->read(RELOCATION_STARTUP + source->a * 4);
    if (source->kind == SOURCE_TAIL) return s->tail;
    at = s->window_at[source->window];
    if (!at) return 0;
    at = relocation_source_of(s, t->windows[source->window].region, at);
    switch (source->kind) {
    case SOURCE_WINDOW:
        return s->window_at[source->window] + source->a;
    case SOURCE_LITERAL:
        return s->read(at + source->a * 4);
    case SOURCE_PAIR: {
        u32 low = s->read(at + source->a * 4), high = s->read(at + source->b * 4);
        return (((high >> 4) & 0xf000U) | (high & 0xfffU)) << 16 | ((low >> 4) & 0xf000U) | (low & 0xfffU);
    }
    case SOURCE_BRANCH: {
        u32 site = at + source->a * 4;
        word = s->read(site);
        if (((word >> 25) & 7) != 5 || (word >> 28) == 0xf) {
            relocation_fail(s, RELOCATION_NO_BRANCH, i, symbol, word);
            return 0;
        }
        u32 target = site + 8 + (u32)(((int)(word << 8)) >> 6);
        if (source->b != RELOCATION_NONE) {
            /* The target is the start of the verify window, and the symbol
               keeps its offset from it. */
            const struct relocation_window *verify = &t->windows[source->b];
            if (!relocation_matches(t, s, source->b, target)) {
                relocation_fail(s, RELOCATION_VERIFY, i, symbol, target);
                return 0;
            }
            return target + (symbol - verify->start);
        }
        return target;
    }
    }
    return 0;
}

static u32 relocation_value(const struct relocation_tables *t, struct relocation_state *s, u32 i)
{
    const struct relocation_source *source = &t->sources[i];
    u32 failures = s->failures, value = relocation_read_source(t, s, i);
    int found = source->kind == SOURCE_STARTUP || source->kind == SOURCE_TAIL || s->window_at[source->window];
    /* A zero address from a found window, such as a MOVW and MOVT pair of 0. */
    if (!value && found && s->failures == failures)
        relocation_fail(s, RELOCATION_OUTSIDE, i, t->symbols[source->symbol], 0);
    return value;
}

/* Resolves every symbol into s->symbol_at. Returns 1 when all resolve. A
   symbol that does not resolve keeps 0 or 1 (relocation_unresolved). */
static int relocation_resolve(const struct relocation_tables *t, struct relocation_state *s)
{
    s->failures = 0;
    for (u32 symbol = 0; symbol < t->symbol_count; ++symbol) s->symbol_at[symbol] = 0;
    if (!relocation_startup(t, s)) return 0;
    relocation_search(t, s);
    u32 tail = RELOCATION_NONE;
    for (u32 i = 0; i < t->source_count; ++i) {
        const struct relocation_source *source = &t->sources[i];
        if(s->required && !s->required[source->symbol]) continue;
        if (source->kind == SOURCE_TAIL) tail = source->symbol;
        u32 value = relocation_value(t, s, i), *slot = &s->symbol_at[source->symbol];
        if (!value) {
            /* The failure is already in the report. */
            *slot = 1;
            continue;
        }
        if (*slot == 1) continue;
        if (*slot && *slot != value) {
            relocation_fail(s, RELOCATION_DISAGREE, source->symbol, t->symbols[source->symbol], value);
            *slot = 1;
            continue;
        }
        *slot = value;
    }
    for (u32 symbol = 0; symbol < t->symbol_count; ++symbol) {
        if(s->required && !s->required[symbol]) continue;
        u32 value = s->symbol_at[symbol];
        if (value <= 1) {
            if (!value) relocation_fail(s, RELOCATION_NOT_FOUND, symbol, t->symbols[symbol], 0);
            continue;
        }
        /* Code and read-only data lie in the image before the .data copy
           source, and data before the heap. A branch target after the image
           would lie where the private code goes. The tail is checked at
           startup. */
        u32 pinned = t->symbols[symbol];
        if (symbol == tail) continue;
        int code = pinned < RELOCATION_DATA_START;
        if (code ? value < RELOCATION_RTOS_BASE || value >= s->data_source
                 : value < RELOCATION_DATA_START || value > s->bss_end) {
            relocation_fail(s, RELOCATION_OUTSIDE, symbol, pinned, value);
            s->symbol_at[symbol] = 1;
        }
    }
    return s->failures == 0;
}

/* Whether `symbol` did not resolve. */
static int relocation_unresolved(const struct relocation_state *s, u32 symbol)
{
    return s->symbol_at[symbol] <= 1;
}

/* After a resolve that failed: the extension points of the symbols that did
   not resolve, from `points` (0 for a symbol that each install needs), into
   *dropped. Returns 0 when a needed symbol did not resolve. */
static int relocation_dropped(const struct relocation_tables *t, const struct relocation_state *s,
                              const u32 *points, u32 *dropped)
{
    *dropped = 0;
    for (u32 symbol = 0; symbol < t->symbol_count; ++symbol) {
        if (!relocation_unresolved(s, symbol)) continue;
        if (!points[symbol]) return 0;
        *dropped |= points[symbol];
    }
    return 1;
}

/* Moves the word at `p` that refers to `symbol`. A branch also moves with the
   patch `site` that holds it. Returns 0 when a branch leaves its range. */
static int relocation_move(u8 *p, int branch, u32 delta, u32 site_delta)
{
    u32 word = (u32)p[0] | (u32)p[1] << 8 | (u32)p[2] << 16 | (u32)p[3] << 24;
    if (branch) {
        int offset = ((int)(word << 8)) >> 8;
        int change = (int)(delta - site_delta) / 4;
        offset += change;
        if (((word >> 25) & 7) != 5 || offset >= (1 << 23) || offset < -(1 << 23)) return 0;
        word = (word & 0xff000000U) | ((u32)offset & 0xffffffU);
    } else {
        word += delta;
    }
    p[0] = (u8)word; p[1] = (u8)(word >> 8); p[2] = (u8)(word >> 16); p[3] = (u8)(word >> 24);
    return 1;
}

#endif
