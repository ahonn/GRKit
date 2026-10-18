#include "abi.h"
#include "menu_labels.h"
/* GRKit installer. Native selection state is read-only. */
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
/* kind 1: a reservation after the decoded RTOS image. BOOT does not reload
   it, so it can keep the previous session's code after a restart. */
struct patch { u32 address, size, kind; const u8 *before; u8 *after; };
/* Stock bytes that statically patched firmware changes. */
struct guard { u32 address, size; const u8 *bytes; };
#include "module_payload.h"

static long syscall6(u32 number, u32 a, u32 b, u32 c, u32 d, u32 e, u32 f)
{
    register u32 r0 __asm__("r0") = a;
    register u32 r1 __asm__("r1") = b;
    register u32 r2 __asm__("r2") = c;
    register u32 r3 __asm__("r3") = d;
    register u32 r4 __asm__("r4") = e;
    register u32 r5 __asm__("r5") = f;
    register u32 r7 __asm__("r7") = number;
    __asm__ volatile("svc 0" : "+r"(r0) : "r"(r1), "r"(r2), "r"(r3),
                     "r"(r4), "r"(r5), "r"(r7) : "memory", "cc");
    return (long)r0;
}
static volatile u8 *mapping;
static u8 profiles[MAX_COUNT][8192], temporary[8192], directory_entries[4096];
static u8 plan[20 + MAX_COUNT * 8192];
static u32 count, size, icon_chunks;
static const char root[] = EFFECTS_PATH;
static const char plan_path[] = PLAN_PATH;
static const char plan_tmp[] = PLAN_TMP_PATH;
static const char lock_path[] = LOCK_PATH;
static const char done_path[] = DONE_PATH;
static const char failed_path[] = FAILED_PATH;
static const char modules_path[] = MODULES_PATH;
static volatile u8 *at(u32 physical) { return mapping + physical - MAP_BASE; }
static u32 get32(const u8 *p) { return p[0] | p[1] << 8 | p[2] << 16 | (u32)p[3] << 24; }
static void put32(u8 *p, u32 v) { for (u32 i=0; i<4; ++i) p[i] = v >> (i*8); }
static u32 word(u32 p) { return *(volatile u32 *)at(p); }
static void store(u32 p, u32 v) { *(volatile u32 *)at(p) = v; }
static int equal(volatile const u8 *a, const u8 *b, u32 n)
{ for (u32 i=0; i<n; ++i) if (a[i] != b[i]) return 0; return 1; }
static void copy(volatile u8 *a, const u8 *b, u32 n)
{ for (u32 i=0; i<n; ++i) a[i] = b[i]; }
static void zero(volatile u8 *a, u32 n) { for (u32 i=0; i<n; ++i) a[i]=0; }
static u32 length(const char *s) { u32 n=0; while (s[n]) ++n; return n; }
static int compare(const u8 *a, const u8 *b)
{ while (*a && *a==*b) {++a; ++b;} return (int)*a-(int)*b; }
static int text_equal(const char *a, const char *b) { return compare((const u8 *)a,(const u8 *)b)==0; }
static void print(const char *s) { syscall6(4,1,(u32)s,length(s),0,0,0); }
static void number(u32 value)
{
    char digits[12]; u32 n=0;
    do { digits[n++]='0'+value%10; value/=10; } while(value);
    for (u32 i=0; i<n/2; ++i) { char x=digits[i]; digits[i]=digits[n-1-i]; digits[n-1-i]=x; }
    digits[n]=0; print(digits);
}
#define GRKIT_GRF_EMBEDDED
#include "grf.c"
#include "catalog.h"
#include "visibility.h"
static char names[MAX_COUNT][CATALOG_NAME_SIZE];
static int read_file(const char *path, u8 *buffer, u32 wanted)
{
    long fd=syscall6(5,(u32)path,0x8800,0,0,0,0);
    if(fd<0) return 0;
    u32 have=0; long got=0;
    while(have<wanted) {
        got=syscall6(3,fd,(u32)(buffer+have),wanted-have,0,0,0);
        if(got<=0) break;
        have+=(u32)got;
    }
    u8 extra; got=syscall6(3,fd,(u32)&extra,1,0,0,0);
    syscall6(6,fd,0,0,0,0,0);
    return have==wanted && got==0;
}
/* A missing MODULES.TXT hides nothing. An unreadable or invalid one stops the
   installation, so a card never shows a menu that its file does not describe. */
static int read_hidden(u32 *hidden)
{
    u8 text[VISIBILITY_FILE_MAX + 1];
    u32 have = 0; long got = 0;
    long fd = syscall6(5, (u32)modules_path, 0x8800, 0, 0, 0, 0);
    *hidden = 0;
    if (fd == -2) return 0;
    if (fd < 0) return 44;
    while (have < sizeof(text)) {
        got = syscall6(3, fd, (u32)(text + have), sizeof(text) - have, 0, 0, 0);
        if (got <= 0) break;
        have += (u32)got;
    }
    syscall6(6, fd, 0, 0, 0, 0, 0);
    if (got < 0) return 44;
    if (have > VISIBILITY_FILE_MAX || !visibility_parse(text, have, hidden)) return 43;
    return 0;
}
static int discover(void)
{
    count=0;
#if defined(GRKIT_NO_MODULES) || defined(GRKIT_NO_IMAGE_CONTROL)
    /* Without Image Control, the target has no use for effect files. */
    return 0;
#endif
    long fd=syscall6(5,(u32)root,0xc000,0,0,0,0);
    if(fd==-2) return 0;
    if(fd<0) return 30;
    int status=0; long got;
    while((got=syscall6(217,fd,(u32)directory_entries,sizeof(directory_entries),0,0,0))>0) {
        u32 offset=0;
        while(offset<(u32)got) {
            const u8 *entry=directory_entries+offset;
            if((u32)got-offset<20) {status=31;break;}
            u32 size=(u32)entry[16]|entry[17]<<8;
            if(size<20 || size>(u32)got-offset) {status=31;break;}
            u32 n=0; while(n<size-19 && entry[19+n]) ++n;
            if(n==size-19) {status=31;break;}
            const char *name=(const char *)entry+19;
            /* A symbolic link or an unusable file does not stop the other effects. */
            if(entry[18]!=10 && catalog_name(name,n)) {
                char path[sizeof(root) + 13]; u32 prefix=length(root);
                copy((u8 *)path,(const u8 *)root,prefix); path[prefix++]='/';
                copy((u8 *)path+prefix,(const u8 *)name,n+1);
                if(read_file(path,temporary,8192) && valid_profile(temporary))
                    count=catalog_offer(profiles,names,count,MAX_COUNT,temporary,name);
            }
            offset+=size;
        }
        if(status) break;
    }
    if(got<0 && !status) status=30;
    syscall6(6,fd,0,0,0,0,0);
    return status;
}
/* Photos record this code as their Image Control tag. It stays stable across
   catalogs and outside the native values 256..267. */
#ifdef GRKIT_NATIVE_TONE_PAGE
/* The target also records native values in this 256-value page. */
#define NATIVE_CODE(code) ((code) >> 8 == GRKIT_NATIVE_TONE_PAGE)
#else
#define NATIVE_CODE(code) 0
#endif
static u32 effect_code(const u8 *id)
{
    u32 hash = 0x811c9dc5;
    while (*id) { hash ^= *id++; hash *= 0x01000193; }
    return 0x8000 | (hash & 0x7fff);
}
static void layout(void)
{
    size = GRKIT_HOST_HEADER_SIZE + count * GRKIT_RECORD_SIZE + GRKIT_PICTURE_SCRATCH;
    icon_chunks = (count + HOST_ICON_COUNT + ICONS_PER_CHUNK - 1) / ICONS_PER_CHUNK;
}
static u32 chunk_count(u32 chunk)
{
    u32 left = count + HOST_ICON_COUNT - chunk * ICONS_PER_CHUNK;
    return left < ICONS_PER_CHUNK ? left : ICONS_PER_CHUNK;
}
static int scan(void)
{
    int status = discover();
    if (status) return status;
    layout();
    put32(plan, 0x314d5247); put32(plan + 4, count);
    put32(plan + 8, size / 4); put32(plan + 12, icon_chunks);
    put32(plan + 16, 20 + count * 8192);
    for (u32 i = 0; i < count; ++i) copy(plan + 20 + i * 8192, profiles[i], 8192);
    /* O_EXCL: the loader resends SCAN until a plan appears, so attempts can overlap. */
    long fd = syscall6(5, (u32)plan_tmp, 0xc1, 0600, 0, 0, 0);
    if (fd == -17) return 3;
    if (fd < 0) return 36;
    u32 done = 0, total = 20 + count * 8192;
    while (done < total) {
        long n = syscall6(4, fd, (u32)(plan + done), total - done, 0, 0, 0);
        if (n <= 0) break;
        done += n;
    }
    if (syscall6(6, fd, 0, 0, 0, 0, 0) < 0 || done < total ||
        syscall6(38, (u32)plan_tmp, (u32)plan_path, 0, 0, 0, 0) < 0) {
        syscall6(10, (u32)plan_tmp, 0, 0, 0, 0, 0);
        return 36;
    }
    print("GR_MODULE_SCANNED "); number(count); print("\n");
    return 0;
}
static int sync_alias(u32 address, u32 n)
{
    __asm__ volatile("dsb sy" ::: "memory");
    return syscall6(0xf0002, (u32)at(address), (u32)at(address + n), 0, 0, 0, 0) == 0;
}
/* A native icon descriptor, then the 40 x 40 pixels. */
#ifdef GRKIT_ICON_DESCRIPTOR_12
/* Width and a valid flag, height, and the pixel offset in words. */
#define ICON_PIXELS 12
#define ICON_HEADER(icon) do { store(icon, 0x00280001); store(icon + 4, 40); \
    store(icon + 8, (icon + ICON_PIXELS - pixels) / 4); } while (0)
#else
/* Height and width, then the pixel offset in words. */
#define ICON_PIXELS 8
#define ICON_HEADER(icon) do { store(icon, 0x00280028); \
    store(icon + 4, (icon + ICON_PIXELS - pixels) / 4); } while (0)
#endif
static int heap(u32 p, u32 n)
{
    return !(p & 3) && p >= HEAP_MIN && n <= HEAP_END - HEAP_MIN && p <= HEAP_END - n;
}
static void publish(struct patch *p, const u8 *bytes)
{
    if (p->size == 4) store(p->address, get32(bytes));
    else copy(at(p->address), bytes, p->size);
    __asm__ volatile("dsb sy" ::: "memory");
}
static int install(void)
{
    u32 hidden;
    int status = discover();
    if (status) return status;
    if ((status = read_hidden(&hidden))) return status;
    layout();
    if (!read_file(plan_path, plan, 20 + count * 8192) || get32(plan) != 0x314d5247 ||
        get32(plan + 4) != count || get32(plan + 8) != size / 4 ||
        get32(plan + 12) != icon_chunks || get32(plan + 16) != 20 + count * 8192) return 37;
    for (u32 i = 0; i < count; ++i)
        if (!equal(plan + 20 + i * 8192, profiles[i], 8192)) return 37;
    if (*(volatile u16 *)at(ARRAY_COUNT) != 1 + icon_chunks || word(ARRAY_TABLE) != size / 4) return 10;
    u32 base = word(ARRAY_TABLE + 4), icon_bases[MAX_CHUNKS];
    if (!heap(base, size)) return 11;
    for (u32 i = 0; i < icon_chunks; ++i) {
        u32 bytes = chunk_count(i) * ICON_STRIDE;
        u32 ptr = word(ARRAY_TABLE + (i + 1) * 8 + 4);
        if (word(ARRAY_TABLE + (i + 1) * 8) != bytes / 4 || !heap(ptr, bytes) ||
            (ptr < base + size && base < ptr + bytes)) return 11;
        for (u32 j = 0; j < i; ++j)
            if (ptr < icon_bases[j] + chunk_count(j) * ICON_STRIDE && icon_bases[j] < ptr + bytes) return 11;
        icon_bases[i] = ptr;
    }
#ifdef GUARD_COUNT
    /* Patched firmware can extend the RTOS image into the private tail
       regions. GRKit then refuses to load instead of overwriting that code. */
    for (u32 i = 0; i < GUARD_COUNT; ++i)
        if (!equal(at(guards[i].address), guards[i].bytes, guards[i].size)) return 45;
#endif
#ifndef GRKIT_NO_MODULES
    if (word(ENGLISH_SLOT) != ORIGINAL_ENGLISH) return 12;
#ifndef GRKIT_NO_IMAGE_CONTROL
    if (!equal(at(STOCK_RECORD), stock_record, 68)) return 13;
    /* Firmware fills four pointers at +0x44 after the image copy. Records copied
       before that would miss them, so an image-copy tail means not ready yet. */
    if (!equal(at(STOCK_RECORD + 68), initialized_tail, 16))
        return equal(at(STOCK_RECORD + 68), stock_record + 68, 16) ? 14 : 13;
#endif
    /* No firmware code refers to the RTOS tail, so earlier GRKit code there is
       overwritten. Every other patch must still find its stock bytes. */
#endif
    for (u32 i = 0; i < PATCH_COUNT; ++i)
        if (!patches[i].kind && !equal(at(patches[i].address), patches[i].before, patches[i].size)) return 16;
#ifndef GRKIT_NO_MODULES
#ifndef GRKIT_NO_IMAGE_CONTROL
    for (u32 i = 0; i < 3; ++i)
        if (!equal(at(table_addresses[i]), stock_tables + i * TABLE_BYTES, TABLE_BYTES)) return 17;
    /* Reject stale native types; never attempt an in-place settings repair. */
    u32 domain = word(SETTINGS_DOMAIN);
    if (!heap(domain, 0x51c)) return 39;
    u32 control = word(domain + 0x518);
    if (!heap(control, 4)) return 39;
    u32 payload = word(control);
    if (!heap(payload, SETTINGS_IC_TYPE + 1) || at(payload + SETTINGS_IC_TYPE)[0] >= IC_TYPE_COUNT) return 39;
#endif
#ifdef ICON_DRAWER_SLOT
    /* The icon drawer exists after the first icon draw. Its first word is the
       base of icon pixel offsets, which can lie outside the mapping. */
    u32 drawer = word(ICON_DRAWER_SLOT);
    if (!heap(drawer, 4)) return 26;
    u32 pixels = word(drawer);
    if ((pixels & 3) || !pixels) return 26;
#else
    u32 pixels = word(ICON_PIXEL_BASE_SLOT);
    if ((pixels & 3) || pixels < MAP_BASE || pixels >= HEAP_MIN) return 26;
#endif
#endif
    zero(at(base), size);
    store(base, count); store(base + 4, base + GRKIT_HOST_HEADER_SIZE);
    store(base + 16, 0x314d5247); store(base + 20, 1);
#ifdef TARGET_HIDDEN
    /* Modules that the target does not implement yet stay hidden. */
    hidden |= TARGET_HIDDEN;
#endif
    store(base + GRKIT_HOST_HIDDEN, hidden);
#ifndef GRKIT_NO_MODULES
    for (u32 i = 0; i < icon_chunks; ++i) {
        zero(at(icon_bases[i]), chunk_count(i) * ICON_STRIDE);
        store(base + GRKIT_HOST_ICON_CHUNKS + i * 4, icon_bases[i]);
    }
#ifndef GRKIT_NO_IMAGE_CONTROL
    for (u32 i = 0; i < count; ++i) {
        u32 record = base + GRKIT_HOST_HEADER_SIZE + i * GRKIT_RECORD_SIZE;
        const u8 *profile = profiles[i];
        copy(at(record), (const u8 *)at(STOCK_RECORD), 84);
        store(record + 4, record + 84);
        copy(at(record + 84), profile + 128, 1556);
        /* Extension: features, then the post matrix, selective color, and the
           reserved zero bytes. */
        copy(at(record + GRKIT_RECORD_FEATURES), profile + 84, 4);
        copy(at(record + GRKIT_RECORD_POST), profile + 88, 40);
        u32 n = length((const char *)profile + 48);
        for (u32 j = 0; j < n; ++j) at(record + GRKIT_RECORD_NAME + j * 2)[0] = profile[48 + j];
        store(record + GRKIT_RECORD_MONOCHROME, get32(profile + 80));
        copy(at(record + GRKIT_RECORD_ID), profile + 16, 32);
        module_summary((volatile u16 *)at(record + GRKIT_RECORD_SUMMARY), profile + 48);
        u32 icon = icon_bases[i / ICONS_PER_CHUNK] + (i % ICONS_PER_CHUNK) * ICON_STRIDE;
        ICON_HEADER(icon);
        copy(at(icon + ICON_PIXELS), profile + 1684, 6400);
        store(record + GRKIT_RECORD_ICON, icon);
        /* Resolve hash collisions in catalog order, and skip native values. */
        u32 code = effect_code(profile + 16), j = 0;
        while (j < i || NATIVE_CODE(code)) {
            if (!NATIVE_CODE(code) && *(volatile u16 *)at(base + GRKIT_HOST_HEADER_SIZE + j * GRKIT_RECORD_SIZE + GRKIT_RECORD_CODE) != code) { ++j; continue; }
            code = 0x8000 | ((code + 1) & 0x7fff);
            j = 0;
        }
        *(volatile u16 *)at(record + GRKIT_RECORD_CODE) = code;
    }
#endif
    /* Host artwork does not consume an effect record or a native settings ID. */
    for (u32 i = 0; i < HOST_ICON_COUNT; ++i) {
        u32 index = count + i;
        u32 icon = icon_bases[index / ICONS_PER_CHUNK] + (index % ICONS_PER_CHUNK) * ICON_STRIDE;
        ICON_HEADER(icon);
        const u8 *artwork = i >= 2 ? module_pixels + (i - 2) * 6400 : i ? plugins_pixels : off_pixels;
        copy(at(icon + ICON_PIXELS), artwork, 6400);
        /* The menu derives the other module tiles from the first Framing tile. */
        if (i <= 2) store(base + (i == 2 ? GRKIT_HOST_FRAMING_ICON : i ? GRKIT_HOST_PLUGINS_ICON : GRKIT_HOST_OFF_ICON), icon);
    }
#endif
    for (u32 i = 0; i < CODE_RELOCATION_COUNT; ++i) {
        u32 patch_index = code_relocations[i * 2], offset = code_relocations[i * 2 + 1];
        put32(patches[patch_index].after + offset, base);
    }
    if (!sync_alias(base, size)) return 21;
    for (u32 i = 0; i < icon_chunks; ++i)
        if (!sync_alias(icon_bases[i], chunk_count(i) * ICON_STRIDE)) return 21;
    /* Publish payloads before entry branches. Cross-domain coherence needs device validation. */
    u32 installed = 0;
    for (; installed < PATCH_COUNT; ++installed) {
        publish(&patches[installed], patches[installed].after);
        if (!sync_alias(patches[installed].address, patches[installed].size)) break;
    }
    if (installed != PATCH_COUNT) {
        int rollback_ok = 1;
        for (u32 i = installed + 1; i > 0; --i) {
            publish(&patches[i - 1], patches[i - 1].before);
            if (!sync_alias(patches[i - 1].address, patches[i - 1].size)) rollback_ok = 0;
        }
        return rollback_ok ? 23 : 24;
    }
    print("GR_MODULE_INSTALLED "); number(count); print("\n");
    return 0;
}
#ifdef MODULE_DIAGNOSTICS
/* Diagnostic builds can select profiles independently of the menu. */
static int select_profile(const char *id)
{
    u32 base = word(ARRAY_TABLE + 4);
    if (!heap(base, GRKIT_HOST_HEADER_SIZE) || word(base + 16) != 0x314d5247 || word(base + 20) != 1) return 41;
    u32 n = word(base), records = word(base + 4), selected = 0;
    if (n > MAX_COUNT || records != base + GRKIT_HOST_HEADER_SIZE || !heap(base, GRKIT_HOST_HEADER_SIZE + n * GRKIT_RECORD_SIZE + GRKIT_PICTURE_SCRATCH)) return 41;
    if (!text_equal(id, "off")) {
        for (u32 i = 0; i < n; ++i) {
            u32 p = records + i * GRKIT_RECORD_SIZE;
            if (equal(at(p + GRKIT_RECORD_ID), (const u8 *)id, length(id) + 1)) { selected = p; break; }
        }
        if (!selected) return 42;
    }
    store(base + 8, selected);
    if (!sync_alias(base, GRKIT_HOST_HEADER_SIZE)) return 21;
    print("GR_MODULE_SELECTED "); print(id); print("\n");
    return 0;
}
#endif
/* Create an empty file only when it is absent. */
/* Create a result file. A nonzero status is written as decimal text, so a
   card from a camera shows why INSTALL failed. */
static int create(const char *path, u32 status)
{
    long fd = syscall6(5, (u32)path, 0xc1, 0600, 0, 0, 0);
    if (fd < 0) return 0;
    if (status) {
        char text[12]; u32 n = sizeof(text) - 1;
        text[n] = '\n';
        do { text[--n] = '0' + status % 10; status /= 10; } while (status);
        syscall6(4, fd, (u32)(text + n), sizeof(text) - n, 0, 0, 0);
    }
    syscall6(6, fd, 0, 0, 0, 0, 0);
    return 1;
}
/* Native state that the camera is still initializing. Other failures are final. */
static int not_ready(int status)
{
    return status == 12 || status == 14 || status == 26 || status == 39;
}
__attribute__((used)) void start(u32 *stack)
{
    u32 argc = stack[0]; char **argv = (char **)(stack + 1); int status = 40;
    if (argc == 2 && text_equal(argv[1], "--scan")) status = scan();
    else if ((argc == 2 && text_equal(argv[1], "--install"))
#ifdef MODULE_DIAGNOSTICS
             || (argc == 3 && text_equal(argv[1], "--select") && length(argv[2]) <= 31)
#endif
    ) {
        long fd = -1;
        /* The lock admits one install attempt at a time and stays after a result. */
        if (argc == 2 && !create(lock_path, 0)) status = 3;
        else if ((fd = syscall6(5, (u32)"/dev/mem", 0x101002, 0, 0, 0, 0)) < 0) status = 1;
        else {
            long mapped = syscall6(192, 0, MAP_SIZE, 3, 1, fd, MAP_BASE >> 12);
            status = 2;
            if ((u32)mapped < 0xfffff001U) {
                mapping = (volatile u8 *)mapped;
                if (argc == 2) status = install();
#ifdef MODULE_DIAGNOSTICS
                else status = select_profile(argv[2]);
#endif
                syscall6(91, (u32)mapping, MAP_SIZE, 0, 0, 0, 0);
            }
            syscall6(6, fd, 0, 0, 0, 0, 0);
        }
        /* The loader polls these files: retry after not-ready, stop after a result. */
        if (argc == 2 && status != 3) {
            if (not_ready(status)) syscall6(10, (u32)lock_path, 0, 0, 0, 0, 0);
            else create(status ? failed_path : done_path, status);
        }
    }
    if (status) { print("GR_MODULE_REJECTED "); number(status); print("\n"); }
    syscall6(1, status, 0, 0, 0, 0, 0); for (;;) {}
}
__attribute__((naked)) void _start(void) { __asm__ volatile("mov r0, sp\nb start"); }
