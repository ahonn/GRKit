/* The RTOS side of the installer: the backend of installer.c built with
 * -DRTOS_NATIVE, which the head of a fast-boot file (head.S) loads and calls.
 *
 * installer.c performs all I/O through syscall6
 * and all camera-memory access through at()/word()/store(), which resolve
 * against the address that mmap returns. This backend:
 *   - serves the card paths (/media/storage1/...) from the SD card through the
 *     RTOS file API (C:\...), so every file the installer writes reaches the
 *     card as on Linux;
 *   - keeps /tmp files in RAM: the whole plan that INSTALL reads after SCAN;
 *     SCAN gives the loader the plan header in the script array
 *     (FAST_PLAN_INDEX), so no plan file reaches the card;
 *   - reads each card file and directory of the start once (the cache), as
 *     nothing but this file changes the card until INSTALL ends;
 *   - keeps the effects and the modules of the start in one card file
 *     (the pack), which the next start reads in one call;
 *   - checks at each INSTALL call that its data did not change since the
 *     last call (data_changed), as the head of the install file checks the
 *     code, then starts INSTALL from the .data of the file and a clear .bss
 *     (data_reset), so a retry after a not-ready status starts as the first
 *     try;
 *   - returns the physical map base from mmap, so at(p) == (u8*)p;
 *   - implements msync as a real cache clean + instruction-cache invalidate;
 *   - routes stdout to the RTOS printf and the exit markers to the RTOS log.
 *
 * The RTOS file system keeps 8.3 names only. A directory is listed once when
 * it is opened; getdents64 and lseek then work on that copy.
 */
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
#define MAP_BASE 0x51000000U

/* The RTOS calls of the firmware build (targets/<camera>-<version>.h). */
#include "target.h"
/* The status word of the head in the script array (FAST_STATUS_INDEX). */
#include "fast.h"
#include "calls.h"
#include "../install_status.h"

typedef int (*printf_fn)(const char *, ...);
typedef void (*log_fn)(u32, u32, u32, u32, const char *, ...);
#define RPRINTF ((printf_fn)RT_PRINTF)
#define RLOG ((log_fn)RT_LOG)

typedef int (*fs_open_fn)(const char *name, u32 mode, u32 amode);
typedef int (*fs_close_fn)(u32 fno);
typedef int (*fs_rw_fn)(u32 fno, void *addr, u32 count, u32 *len);
typedef int (*fs_create_fn)(const char *name, u32 amode);
typedef int (*fs_name_fn)(const char *name);
typedef int (*fs_opendir_fn)(const char *name, u32 *dno);
typedef int (*fs_readdir_fn)(u32 dno, void *entry);
typedef int (*fs_closedir_fn)(u32 dno);
typedef int (*fs_rename_fn)(const char *src, const char *dst);
typedef int (*fs_stat_fn)(const char *name, void *out);
#define FS_OPEN     ((fs_open_fn)RT_FS_OPEN)
#define FS_CLOSE    ((fs_close_fn)RT_FS_CLOSE)
#define FS_READ     ((fs_rw_fn)RT_FS_READ)
#define FS_WRITE    ((fs_rw_fn)RT_FS_WRITE)
#define FS_CREATE   ((fs_create_fn)RT_FS_CREATE)
#define FS_REMOVE   ((fs_name_fn)RT_FS_REMOVE)
#define FS_MKDIR    ((fs_name_fn)RT_FS_MKDIR)
#define FS_OPENDIR  ((fs_opendir_fn)RT_FS_OPENDIR)
#define FS_READDIR  ((fs_readdir_fn)RT_FS_READDIR)
#define FS_CLOSEDIR ((fs_closedir_fn)RT_FS_CLOSEDIR)
#define FS_RENAME   ((fs_rename_fn)RT_FS_RENAME)
#define FS_SIZE     ((fs_stat_fn)RT_FS_STAT_SIZE)   /* out[2] = size, 0 ok */
/* Open modes of the RTOS file system. */
#define MODE_RDWR 0U        /* read and write in place */
#define MODE_TRUNCATE 0x200U

#define ENOENT_ERROR (-2L)
#define EIO_ERROR (-5L)
#define EBADF_ERROR (-9L)
#define EEXIST_ERROR (-17L)
#define EMFILE_ERROR (-24L)
#define ENOSYS_ERROR (-38L)

static u32 slen(const char *s) { u32 n = 0; while (s[n]) ++n; return n; }
static int seq(const char *a, const char *b) { while (*a && *a == *b) { ++a; ++b; } return *a == *b; }
static int prefix(const char *s, const char *p) { while (*p) if (*s++ != *p++) return 0; return 1; }
/* The RTOS file system ignores the case of names. */
static int same_name(const char *a, const char *b)
{
    for (;; ++a, ++b) {
        char x = *a >= 'a' && *a <= 'z' ? (char)(*a - 32) : *a, y = *b >= 'a' && *b <= 'z' ? (char)(*b - 32) : *b;
        if (x != y) return 0;
        if (!x) return 1;
    }
}
static void copy_name(char *to, const char *from) { while ((*to++ = *from++)) {} }

static void putbytes(const char *s, u32 n)
{
    char line[256]; u32 i = 0;
    while (i < n) { u32 k = 0; while (k < 255 && i < n) line[k++] = s[i++]; line[k] = 0; RPRINTF("%s", line); }
}

/* "/media/storage1/A/B" -> "C:\A\B". Returns 0 for any other path. */
#define CARD_ROOT "/media/storage1"
#define DOS_SIZE 160U
static int dos_path(const char *path, char *out)
{
    if (!prefix(path, CARD_ROOT)) return 0;
    const char *rest = path + sizeof CARD_ROOT - 1;
    if (*rest && *rest != '/') return 0;
    u32 n = 0;
    out[n++] = 'C'; out[n++] = ':';
    if (!*rest) out[n++] = '\\';
    for (; *rest; ++rest) {
        if (n + 1 >= DOS_SIZE) return 0;
        out[n++] = *rest == '/' ? '\\' : *rest;
    }
    out[n] = 0;
    return 1;
}

/* ---- RAM files (/tmp) ---------------------------------------------------- */
/* The only /tmp file is the plan (FAST_PLAN_FILE). Its largest size is that
   of the plan buffer of installer.c, 354 KB with 40 effects and 64 listed. A
   longer plan is a short write, so SCAN fails and the loader uses Linux. */
#define RAM_FILES 1U
#define RAM_CAP (360U * 1024U)
#define PATH_SIZE 96U
struct ram_file { char path[PATH_SIZE]; u32 size; int present; u8 bytes[RAM_CAP]; };
static struct ram_file ram[RAM_FILES] __attribute__((section(".fast_keep")));
static struct ram_file *ram_find(const char *path)
{
    for (u32 i = 0; i < RAM_FILES; ++i) if (ram[i].present && seq(ram[i].path, path)) return &ram[i];
    return 0;
}
static struct ram_file *ram_make(const char *path)
{
    struct ram_file *f = ram_find(path);
    if (!f) {
        if (slen(path) >= PATH_SIZE) return 0;
        for (u32 i = 0; i < RAM_FILES && !f; ++i) if (!ram[i].present) f = &ram[i];
        if (!f) return 0;
        u32 i = 0; for (; path[i]; ++i) f->path[i] = path[i]; f->path[i] = 0;
        f->present = 1;
    }
    f->size = 0;
    return f;
}

/* ---- directory listings --------------------------------------------------- */
#define LIST_NAMES 256U
#define NAME_SIZE 16U    /* 8.3 names */
/* With `meta`, each entry also has its size, the stamp of its times, and
   whether it is a directory (list_directory). */
struct listing {
    u32 count; int meta;
    char names[LIST_NAMES][NAME_SIZE];
    u32 size[LIST_NAMES], stamp[LIST_NAMES];
    u8 directory[LIST_NAMES];
};
#define LISTINGS 4U
static struct listing listings[LISTINGS];
static int listing_used[LISTINGS];
static u8 dir_entry[2048] __attribute__((aligned(8)));

/* ---- the cache of the start --------------------------------------------------
   SCAN reads each effect three times and each module twice, and INSTALL
   reads the modules again. On a camera each RTOS file call is a round trip to
   the file-system task and each read is a card command, so the first open
   reads a whole file into the cache, and later opens read the cache. The
   cache also keeps directory listings and the paths that do not exist. A
   write, a removal, a rename, or a new directory removes what it can change.
   The cache lasts from SCAN to INSTALL (.fast_keep). */
/* The official modules and 40 effects fit. A file that does not fit reads
   from the card, as without the cache. */
#define CACHE_BYTES (768U * 1024U)
#define CACHE_FILES 96U
#define CACHE_MISSING 32U
#define CACHE_LISTINGS 8U
/* `listed`: the folder listing gave the size and the stamp of the file. */
struct cached_file { char dos[DOS_SIZE]; u32 offset, size, stamp; u8 listed, packed; };
struct cached_listing { char dos[DOS_SIZE]; struct listing list; };
struct cache {
    u32 used, files, missing, listings;
    struct cached_file file[CACHE_FILES];
    char missing_path[CACHE_MISSING][DOS_SIZE];
    struct cached_listing listing[CACHE_LISTINGS];
    u8 bytes[CACHE_BYTES] __attribute__((aligned(8)));
};
static struct cache cache __attribute__((section(".fast_keep")));
static struct cached_file *cache_file(const char *dos)
{
    for (u32 i = 0; i < cache.files; ++i) if (same_name(cache.file[i].dos, dos)) return &cache.file[i];
    return 0;
}
static int cache_missing(const char *dos)
{
    for (u32 i = 0; i < cache.missing; ++i) if (same_name(cache.missing_path[i], dos)) return 1;
    return 0;
}
static void cache_note_missing(const char *dos)
{
    if (cache.missing < CACHE_MISSING && slen(dos) < DOS_SIZE) copy_name(cache.missing_path[cache.missing++], dos);
}
static struct listing *cache_listing(const char *dos)
{
    for (u32 i = 0; i < cache.listings; ++i) if (same_name(cache.listing[i].dos, dos)) return &cache.listing[i].list;
    return 0;
}
/* A change of `dos`: its content, the listings, and the missing paths can
   be other now. The bytes of a removed file stay unused until power-off. */
static void cache_forget(const char *dos)
{
    struct cached_file *f = cache_file(dos);
    /* Field by field: a struct copy would call memcpy, which this file has not. */
    if (f) {
        const struct cached_file *last = &cache.file[--cache.files];
        copy_name(f->dos, last->dos);
        f->offset = last->offset;
        f->size = last->size;
        f->stamp = last->stamp;
        f->listed = last->listed;
        f->packed = last->packed;
    }
    cache.missing = 0;
    cache.listings = 0;
}
static int list_directory(const char *dos, struct listing *l);
/* The cached listing of the folder of `dos`, or 0. With `list`, the folder
   is listed into the cache when it is not there. */
static struct listing *folder_listing(const char *dos, int list, const char **name)
{
    u32 cut = 0;
    for (u32 i = 0; dos[i]; ++i) if (dos[i] == '\\') cut = i;
    if (cut < 3) return 0;
    char folder[DOS_SIZE];
    for (u32 i = 0; i < cut; ++i) folder[i] = dos[i];
    folder[cut] = 0;
    *name = dos + cut + 1;
    struct listing *l = cache_listing(folder);
    if (l || !list || cache.listings >= CACHE_LISTINGS || cache_missing(folder)) return l;
    struct cached_listing *c = &cache.listing[cache.listings];
    if (list_directory(folder, &c->list)) { cache_note_missing(folder); return 0; }
    copy_name(c->dos, folder);
    ++cache.listings;
    return &c->list;
}
static int listed_file(const char *dos, int list, u32 *size, u32 *stamp);

/* ---- the pack ----------------------------------------------------------------
   The effects and the modules of a start, in one card file (FAST_PACK_FILE)
   that the next start reads in one call. A packed file serves an open only
   while the listing of its folder gives the same size and stamp, so a
   changed file reads from the card. After a successful INSTALL, the pack is
   written again when its files differ from those of the start. A sum covers
   the whole pack, so a pack that a power loss cut is not used. */
#define PACK_MAGIC 0x504b5247U  /* "GRKP" */
#define PACK_VERSION 1U
#define PACK_FILES 64U
struct pack_entry { char dos[DOS_SIZE]; u32 size, stamp, offset; };
struct pack_header { u32 magic, version, count, sum, bytes; struct pack_entry entry[PACK_FILES]; };
static const char *const pack_folders[] = FAST_PACK_FOLDERS;
/* The pack that this start read, or 0. */
static const struct pack_header *pack __attribute__((section(".fast_keep")));
static int pack_tried __attribute__((section(".fast_keep")));
static u32 pack_sum(u32 sum, const u8 *p, u32 n)
{
    for (u32 i = 0; i < n; ++i) { sum ^= p[i]; sum *= 0x01000193U; }
    return sum;
}
/* A file of a pack folder: the folder, then a backslash and the name. */
static int pack_folder(const char *dos)
{
    for (u32 k = 0; k < sizeof pack_folders / sizeof pack_folders[0]; ++k) {
        const char *f = pack_folders[k];
        u32 n = slen(f), i = 0;
        while (i < n) {
            char x = dos[i], y = f[i];
            if (x >= 'a' && x <= 'z') x = (char)(x - 32);
            if (y >= 'a' && y <= 'z') y = (char)(y - 32);
            if (x != y) break;
            ++i;
        }
        if (i == n && dos[n] == '\\' && dos[n + 1]) return 1;
    }
    return 0;
}
static void pack_load(void)
{
    if (pack_tried) return;
    pack_tried = 1;
    u32 st[16];
    if (FS_SIZE(FAST_PACK_FILE, st)) return;
    u32 size = st[2], at = cache.used, len = 0;
    if (size < sizeof(struct pack_header) || size > CACHE_BYTES - at) return;
    int f = FS_OPEN(FAST_PACK_FILE, MODE_RDWR, 0);
    if (f < 0) return;
    int failed = FS_READ((u32)f, cache.bytes + at, size, &len);
    FS_CLOSE((u32)f);
    if (failed) return;
    struct pack_header *h = (struct pack_header *)(cache.bytes + at);
    if (h->magic != PACK_MAGIC || h->version != PACK_VERSION || h->count > PACK_FILES ||
        h->bytes != size - sizeof(struct pack_header)) return;
    for (u32 i = 0; i < h->count; ++i) {
        const struct pack_entry *e = &h->entry[i];
        if (e->offset > h->bytes || e->size > h->bytes - e->offset || !e->dos[0] || e->dos[DOS_SIZE - 1]) return;
    }
    u32 sum = h->sum;
    h->sum = 0;
    int valid = pack_sum(0x811c9dc5U, cache.bytes + at, size) == sum;
    h->sum = sum;
    if (!valid) return;
    pack = h;
    cache.used = (at + size + 7U) & ~7U;
}
/* The packed file `dos` with this size and stamp, entered in the cache, or 0. */
static struct cached_file *pack_file(const char *dos, u32 size, u32 stamp)
{
    pack_load();
    if (!pack || cache.files >= CACHE_FILES) return 0;
    for (u32 i = 0; i < pack->count; ++i) {
        const struct pack_entry *e = &pack->entry[i];
        if (e->size != size || e->stamp != stamp || !same_name(e->dos, dos)) continue;
        struct cached_file *c = &cache.file[cache.files++];
        copy_name(c->dos, dos);
        c->offset = (u32)((const u8 *)(pack + 1) - cache.bytes) + e->offset;
        c->size = size;
        c->stamp = stamp;
        c->listed = 1;
        c->packed = 1;
        return c;
    }
    return 0;
}
/* Write the pack again when the files of the pack folders that this start
   read differ from those of the pack that it read. */
static struct pack_header pack_next;
static const u8 pack_zero[8];
static void pack_save(void)
{
    u32 n = 0, bytes = 0, same = pack != 0;
    for (u32 i = 0; i < cache.files && n < PACK_FILES; ++i) {
        const struct cached_file *c = &cache.file[i];
        if (!c->listed || !pack_folder(c->dos)) continue;
        struct pack_entry *e = &pack_next.entry[n++];
        for (u32 k = 0; k < DOS_SIZE; ++k) e->dos[k] = 0;
        copy_name(e->dos, c->dos);
        e->size = c->size;
        e->stamp = c->stamp;
        e->offset = bytes;
        bytes = (bytes + c->size + 7U) & ~7U;
        same &= c->packed;
    }
    if (!n || (same && n == pack->count)) return;
    if (sizeof(struct pack_header) + bytes > CACHE_BYTES) return;
    for (u32 i = n; i < PACK_FILES; ++i)
        for (u32 k = 0; k < sizeof pack_next.entry[i]; ++k) ((u8 *)&pack_next.entry[i])[k] = 0;
    pack_next.magic = PACK_MAGIC;
    pack_next.version = PACK_VERSION;
    pack_next.count = n;
    pack_next.bytes = bytes;
    pack_next.sum = 0;
    u32 sum = pack_sum(0x811c9dc5U, (const u8 *)&pack_next, sizeof pack_next);
    for (u32 i = 0, k = 0; i < cache.files && k < n; ++i) {
        const struct cached_file *c = &cache.file[i];
        if (!c->listed || !pack_folder(c->dos)) continue;
        sum = pack_sum(sum, cache.bytes + c->offset, c->size);
        sum = pack_sum(sum, pack_zero, ((c->size + 7U) & ~7U) - c->size);
        ++k;
    }
    pack_next.sum = sum;
    /* A cut write leaves a pack with another sum, which the next start does
       not use. */
    u32 st[16];
    if (FS_SIZE(FAST_PACK_FILE, st)) {
        int f = FS_CREATE(FAST_PACK_FILE, 0);
        if (f < 0) return;
        FS_CLOSE((u32)f);
    }
    int f = FS_OPEN(FAST_PACK_FILE, MODE_TRUNCATE, 0);
    if (f < 0) return;
    u32 len = 0;
    int failed = FS_WRITE((u32)f, &pack_next, sizeof pack_next, &len);
    for (u32 i = 0, k = 0; !failed && i < cache.files && k < n; ++i) {
        const struct cached_file *c = &cache.file[i];
        if (!c->listed || !pack_folder(c->dos)) continue;
        u32 pad = ((c->size + 7U) & ~7U) - c->size;
        failed = (c->size && FS_WRITE((u32)f, (void *)(cache.bytes + c->offset), c->size, &len)) ||
                 (pad && FS_WRITE((u32)f, (void *)pack_zero, pad, &len));
        ++k;
    }
    FS_CLOSE((u32)f);
}

/* A file of a cached listing with sizes: 1 with its size and stamp, 0 when
   the listing does not have it, or -1 without such a listing. */
static int listed_file(const char *dos, int list, u32 *size, u32 *stamp)
{
    const char *name = 0;
    struct listing *l = folder_listing(dos, list, &name);
    if (!l || !l->meta) return -1;
    for (u32 i = 0; i < l->count; ++i)
        if (same_name(l->names[i], name)) {
            if (l->directory[i]) return -1;
            *size = l->size[i];
            *stamp = l->stamp[i];
            return 1;
        }
    return l->count < LIST_NAMES ? 0 : -1;
}

/* The entry that readdir fills: the name first, then the attribute byte, the
   size, and the times (FAT date and time words). The listing uses the size
   and the times only if each "." entry has the directory attribute, or,
   without "." entries (exFAT), if entries_have_layout agrees. Another
   layout falls back to a status call for each file. */
#define ENTRY_ATTRIBUTE 779U
#define ENTRY_SIZE 784U
#define ENTRY_TIMES 800U
#define ATTRIBUTE_DIRECTORY 0x10U
static u32 entry_word(u32 offset)
{
    return (u32)dir_entry[offset] | (u32)dir_entry[offset + 1] << 8 | (u32)dir_entry[offset + 2] << 16 |
           (u32)dir_entry[offset + 3] << 24;
}
/* Whether the entries of a listing without "." entries, as exFAT gives, have
   the layout above: 0 not known in this start, 1 yes, 2 no. The first such
   listing with a file compares the size of its entry with a status call,
   and needs a stamp other than zero. */
static int entry_layout __attribute__((section(".fast_keep")));
static int entries_have_layout(const char *dos, const struct listing *l)
{
    for (u32 i = 0; !entry_layout && i < l->count; ++i) {
        u32 n = slen(l->names[i]), d = slen(dos);
        /* A name that filled the field can be cut. */
        if (l->directory[i] || !l->size[i] || n + 1 >= NAME_SIZE || d + 1 + n >= DOS_SIZE) continue;
        char path[DOS_SIZE];
        copy_name(path, dos);
        path[d] = '\\';
        copy_name(path + d + 1, l->names[i]);
        u32 st[16];
        entry_layout = FS_SIZE(path, st) == 0 && st[2] == l->size[i] && l->stamp[i] ? 1 : 2;
    }
    return entry_layout == 1;
}
static int list_directory(const char *dos, struct listing *l)
{
    u32 dno = 0;
    int dot = 0, dot_directory = 1;
    if (FS_OPENDIR(dos, &dno)) return -1;
    l->count = 0;
    for (;;) {
        for (u32 i = 0; i < sizeof dir_entry; ++i) dir_entry[i] = 0;
        if (FS_READDIR(dno, dir_entry)) break;
        int directory = (dir_entry[ENTRY_ATTRIBUTE] & ATTRIBUTE_DIRECTORY) != 0;
        if (dir_entry[0] == '.' && !dir_entry[1]) { dot = 1; dot_directory &= directory; }
        if (l->count >= LIST_NAMES) continue;
        u32 n = 0;
        while (n + 1 < NAME_SIZE && dir_entry[n]) { l->names[l->count][n] = (char)dir_entry[n]; ++n; }
        l->names[l->count][n] = 0;
        l->directory[l->count] = (u8)directory;
        l->size[l->count] = entry_word(ENTRY_SIZE);
        /* Not the access date, which a read can change. */
        l->stamp[l->count] = entry_word(ENTRY_TIMES) ^ (entry_word(ENTRY_TIMES + 8) * 0x9e3779b1U) ^
                             (entry_word(ENTRY_TIMES + 12) * 0x85ebca6bU);
        ++l->count;
    }
    FS_CLOSEDIR(dno);
    l->meta = dot ? dot_directory : entries_have_layout(dos, l);
    return 0;
}

/* ---- open-file table ------------------------------------------------------ */
enum kind { FREE, CARD_FILE, CARD_DIR, RAM_FILE, CACHED_FILE, MEMORY };
struct open_file {
    enum kind kind; u32 fno; u32 size; u32 position; int writing;
    struct ram_file *ram; const u8 *bytes; struct listing *list; int own_list;
};
#define OPEN_FILES 12U
#define FIRST_FD 3U
static struct open_file open_files[OPEN_FILES];

static struct open_file *opened(u32 fd)
{
    if (fd < FIRST_FD || fd - FIRST_FD >= OPEN_FILES || open_files[fd - FIRST_FD].kind == FREE) return 0;
    return &open_files[fd - FIRST_FD];
}
static long slot(void)
{
    for (u32 i = 0; i < OPEN_FILES; ++i) if (open_files[i].kind == FREE) return (long)i;
    return -1;
}
static u32 card_size(const char *dos, int *exists)
{
    /* A cached listing of the folder answers without a call. */
    u32 size = 0, stamp = 0;
    int listed = listed_file(dos, 0, &size, &stamp);
    if (listed >= 0) { *exists = listed; return size; }
    /* The file status: 64 bytes, as the head and SYS_memovr give it. The
       call fills it, so it has no initializer, which would need memset. */
    u32 st[16];
    *exists = FS_SIZE(dos, st) == 0;
    return *exists ? st[2] : 0;
}

/* Linux open flags that installer.c uses:
   0x8800 = O_RDONLY|O_CLOEXEC, 0xc000 = O_DIRECTORY|O_CLOEXEC,
   0xc1 = O_WRONLY|O_CREAT|O_EXCL, 0x241 = O_WRONLY|O_CREAT|O_TRUNC,
   0x101002 = /dev/mem O_RDWR|O_SYNC. */
static long do_open(const char *path, u32 flags)
{
    long s = slot();
    if (s < 0) return EMFILE_ERROR;
    struct open_file *o = &open_files[s];
    for (u32 i = 0; i < sizeof *o; ++i) ((u8 *)o)[i] = 0;
    char dos[DOS_SIZE];
    if (seq(path, "/dev/mem")) {
        o->kind = MEMORY;
    } else if (prefix(path, "/tmp/")) {
        if (flags == 0x8800) {
            o->ram = ram_find(path);
            if (!o->ram) return ENOENT_ERROR;
        } else if (flags == 0xc1 || flags == 0x241) {
            if (flags == 0xc1 && ram_find(path)) return EEXIST_ERROR;
            o->ram = ram_make(path);
            if (!o->ram) return EIO_ERROR;
            o->writing = 1;
        } else {
            return EBADF_ERROR;
        }
        o->kind = RAM_FILE;
    } else if (dos_path(path, dos)) {
        int exists = 0;
        if (flags == 0xc000) {
            o->list = cache_listing(dos);
            if (!o->list && cache_missing(dos)) return ENOENT_ERROR;
            if (!o->list && cache.listings < CACHE_LISTINGS && slen(dos) < DOS_SIZE) {
                struct cached_listing *c = &cache.listing[cache.listings];
                if (list_directory(dos, &c->list)) { cache_note_missing(dos); return ENOENT_ERROR; }
                copy_name(c->dos, dos);
                ++cache.listings;
                o->list = &c->list;
            }
            if (!o->list) {
                u32 k = 0;
                while (k < LISTINGS && listing_used[k]) ++k;
                if (k == LISTINGS) return EMFILE_ERROR;
                if (list_directory(dos, &listings[k])) return ENOENT_ERROR;
                listing_used[k] = 1;
                o->list = &listings[k];
                o->own_list = 1;
            }
            o->kind = CARD_DIR;
        } else if (flags == 0x8800) {
            struct cached_file *c = cache_file(dos);
            if (c) {
                o->bytes = cache.bytes + c->offset;
                o->size = c->size;
                o->kind = CACHED_FILE;
                return (long)(s + FIRST_FD);
            }
            if (cache_missing(dos)) return ENOENT_ERROR;
            u32 listed_size = 0, stamp = 0;
            int listed = listed_file(dos, 0, &listed_size, &stamp);
            if (listed > 0 && pack_folder(dos) && (c = pack_file(dos, listed_size, stamp))) {
                o->bytes = cache.bytes + c->offset;
                o->size = c->size;
                o->kind = CACHED_FILE;
                return (long)(s + FIRST_FD);
            }
            o->size = card_size(dos, &exists);
            if (!exists) { cache_note_missing(dos); return ENOENT_ERROR; }
            int f = FS_OPEN(dos, MODE_RDWR, 0);
            if (f < 0) return ENOENT_ERROR;
            /* The whole file in one read, if the cache has room. After a
               failed read, the file reads as without the cache. */
            u32 at = cache.used, len = 0;
            int room = cache.files < CACHE_FILES && slen(dos) < DOS_SIZE && o->size <= CACHE_BYTES - at;
            if (room && (!o->size || !FS_READ((u32)f, cache.bytes + at, o->size, &len))) {
                FS_CLOSE((u32)f);
                c = &cache.file[cache.files++];
                copy_name(c->dos, dos);
                c->offset = at;
                c->size = o->size;
                c->stamp = stamp;
                c->listed = listed > 0 && listed_size == o->size;
                c->packed = 0;
                cache.used = (at + o->size + 7U) & ~7U;
                o->bytes = cache.bytes + at;
                o->kind = CACHED_FILE;
                return (long)(s + FIRST_FD);
            }
            if (room) {
                FS_CLOSE((u32)f);
                f = FS_OPEN(dos, MODE_RDWR, 0);
                if (f < 0) return ENOENT_ERROR;
            }
            o->fno = (u32)f;
            o->kind = CARD_FILE;
        } else if (flags == 0xc1 || flags == 0x241) {
            cache_forget(dos);
            card_size(dos, &exists);
            if (flags == 0xc1 && exists) return EEXIST_ERROR;
            if (!exists) {
                int f = FS_CREATE(dos, 0);
                if (f < 0) return ENOENT_ERROR;
                FS_CLOSE((u32)f);
            }
            int f = FS_OPEN(dos, MODE_TRUNCATE, 0);
            if (f < 0) return EIO_ERROR;
            o->fno = (u32)f;
            o->writing = 1;
            o->kind = CARD_FILE;
        } else {
            return EBADF_ERROR;
        }
    } else {
        return ENOENT_ERROR;
    }
    return (long)(s + FIRST_FD);
}

/* getdents64 records (runtime/directory.h layout: name at offset 19). */
#include "../directory.h"
static long list_entries(struct open_file *o, u8 *buf, u32 size)
{
    u32 used = 0;
    while (o->position < o->list->count) {
        const char *name = o->list->names[o->position];
        u32 n = slen(name);
        u32 record = (DIRECTORY_NAME + n + 1 + 7) & ~7U;
        if (used + record > size) break;
        u8 *e = buf + used;
        for (u32 i = 0; i < record; ++i) e[i] = 0;
        e[16] = (u8)record; e[17] = (u8)(record >> 8); e[18] = 8;
        for (u32 i = 0; i < n; ++i) e[DIRECTORY_NAME + i] = (u8)name[i];
        used += record; ++o->position;
    }
    return (long)used;
}

/* ---- cache maintenance: clean D-cache + invalidate I-cache for [a, a+n) -- */
static void cache_sync(u32 a, u32 n)
{
    for (u32 p = a & ~31U; p < a + n; p += 32)
        __asm__ volatile("mcr p15, 0, %0, c7, c11, 1" :: "r"(p) : "memory"); /* DCCMVAU */
    __asm__ volatile("dsb sy" ::: "memory");
    __asm__ volatile("mcr p15, 0, %0, c7, c1, 0" :: "r"(0) : "memory");       /* ICIALLUIS */
    __asm__ volatile("mcr p15, 0, %0, c7, c1, 6" :: "r"(0) : "memory");       /* BPIALLIS */
    __asm__ volatile("dsb sy\nisb" ::: "memory");
}

/* ---- the syscall dispatcher installer.c calls ---------------------------- */
long syscall6(u32 number, u32 a, u32 b, u32 c, u32 d, u32 e, u32 f);
long syscall6(u32 number, u32 a, u32 b, u32 c, u32 d, u32 e, u32 f)
{
    (void)d; (void)e; (void)f;
    struct open_file *o;
    char dos[DOS_SIZE], dos2[DOS_SIZE];
    switch (number) {
    case 1: /* exit: not reachable under RTOS_NATIVE (the entry returns) */
        return 0;
    case 3: /* read */
        o = opened(a);
        if (!o) return EBADF_ERROR;
        if (o->kind == RAM_FILE || o->kind == CACHED_FILE) {
            const u8 *from = o->kind == RAM_FILE ? o->ram->bytes : o->bytes;
            u32 size = o->kind == RAM_FILE ? o->ram->size : o->size;
            u32 left = size - o->position, k = left < c ? left : c;
            for (u32 i = 0; i < k; ++i) ((u8 *)b)[i] = from[o->position + i];
            o->position += k; return (long)k;
        }
        if (o->kind != CARD_FILE || o->writing) return EBADF_ERROR;
        { u32 left = o->size - o->position, k = left < c ? left : c, len = 0;
          /* The RTOS read reports the requested count, so read exactly what is left. */
          if (k && FS_READ(o->fno, (void *)b, k, &len)) return EIO_ERROR;
          o->position += k; return (long)k; }
    case 4: /* write: fd 1 = stdout */
        if (a == 1 || a == 2) { putbytes((const char *)b, c); return (long)c; }
        o = opened(a);
        if (!o || !o->writing) return EBADF_ERROR;
        if (o->kind == RAM_FILE) {
            u32 room = RAM_CAP - o->position, k = room < c ? room : c;
            for (u32 i = 0; i < k; ++i) o->ram->bytes[o->position + i] = ((const u8 *)b)[i];
            o->position += k; if (o->position > o->ram->size) o->ram->size = o->position;
            return (long)k;
        }
        { u32 len = 0;
          if (c && FS_WRITE(o->fno, (void *)b, c, &len)) return EIO_ERROR;
          o->position += c; return (long)c; }
    case 5: /* open */
        return do_open((const char *)a, b);
    case 6: /* close */
        o = opened(a); if (!o) return EBADF_ERROR;
        if (o->kind == CARD_FILE) FS_CLOSE(o->fno);
        if (o->kind == CARD_DIR && o->own_list) for (u32 k = 0; k < LISTINGS; ++k) if (&listings[k] == o->list) listing_used[k] = 0;
        o->kind = FREE;
        return 0;
    case 10: /* unlink */
        if (prefix((const char *)a, "/tmp/")) { struct ram_file *g = ram_find((const char *)a); if (!g) return ENOENT_ERROR; g->present = 0; return 0; }
        if (!dos_path((const char *)a, dos)) return ENOENT_ERROR;
        if (cache_missing(dos)) return ENOENT_ERROR;
        cache_forget(dos);
        return FS_REMOVE(dos) ? ENOENT_ERROR : 0;
    case 19: /* lseek: directories only (rewind, tell, skip) */
        o = opened(a); if (!o || o->kind != CARD_DIR || c > 1) return EBADF_ERROR;
        { long p = (c ? (long)o->position : 0) + (long)(int)b;
          if (p < 0) return EBADF_ERROR;
          o->position = (u32)p; return p; }
    case 217: /* getdents64 */
        o = opened(a); if (!o || o->kind != CARD_DIR) return EBADF_ERROR;
        return list_entries(o, (u8 *)b, c);
    case 38: /* rename: Linux replaces the destination, the RTOS does not */
        if (!dos_path((const char *)a, dos) || !dos_path((const char *)b, dos2)) return ENOENT_ERROR;
        cache_forget(dos);
        cache_forget(dos2);
        FS_REMOVE(dos2);
        return FS_RENAME(dos, dos2) ? ENOENT_ERROR : 0;
    case 39: /* mkdir */
        if (!dos_path((const char *)a, dos)) return ENOENT_ERROR;
        if (cache_listing(dos)) return EEXIST_ERROR;
        cache_forget(dos);
        if (!FS_MKDIR(dos)) return 0;
        /* As on Linux, an existing directory is EEXIST: the installer writes
           the report only after a new or an existing data folder. */
        { u32 dno = 0;
          if (FS_OPENDIR(dos, &dno)) return EIO_ERROR;
          FS_CLOSEDIR(dno);
          return EEXIST_ERROR; }
    case 91: /* munmap */
        return 0;
    case 192: /* mmap: physical identity map so at(p) == (u8*)p */
        return (long)MAP_BASE;
    case 0xf0002: /* msync alias -> real cache maintenance over [a, b) */
        cache_sync(a, b - a);
        return 0;
    case FAST_FILE_STAMP: /* the size and the stamp of the times of a file, from its listing */
        if (!dos_path((const char *)a, dos)) return ENOENT_ERROR;
        { u32 size = 0, stamp = 0;
          int listed = listed_file(dos, 1, &size, &stamp);
          if (listed < 0) return ENOSYS_ERROR;
          if (!listed) return ENOENT_ERROR;
          ((u32 *)b)[0] = size;
          ((u32 *)b)[1] = stamp;
          return 0; }
    default:
        return -38;
    }
}

extern int grkit_install_entry(u32 argc, char **argv);

static void reset_files(void)
{
    for (u32 i = 0; i < OPEN_FILES; ++i) {
        if (open_files[i].kind == CARD_FILE) FS_CLOSE(open_files[i].fno);
        open_files[i].kind = FREE;
    }
    for (u32 k = 0; k < LISTINGS; ++k) listing_used[k] = 0;
}

/* The installer runs on its own stack: the stack of the script task is too
   small for it. */
#define STACK_SIZE (128U * 1024U)
static u8 stack[STACK_SIZE] __attribute__((aligned(16)));
__attribute__((naked, noinline)) static int on_stack(int (*fn)(u32 *), u32 *arg, void *top)
{
    __asm__ volatile(
        "push {r4, lr}\n"
        "mov r4, sp\n"
        "mov sp, r2\n"
        "mov r3, r0\n"
        "mov r0, r1\n"
        "blx r3\n"
        "mov sp, r4\n"
        "pop {r4, pc}\n");
}

/* ---- the data of the file between calls ----------------------------------
   Nothing but SCAN and INSTALL changes the data of the file (.data and .bss,
   from _code_end to _data_end in fast.ld), and the loader script calls them
   one at a time. So after each call the data has a sum for each block, and
   INSTALL runs only if no block changed. A change means that other code
   wrote into this memory between the calls. */
#define DATA_BLOCK (64U * 1024U)
#define DATA_BLOCKS 96U
/* The most bytes of .data (fast.ld checks it). */
#define DATA_COPY (80U * 1024U)
extern u32 _code_end[], _edata[], _keep_start[], _data_end[];
static u32 data_sums[DATA_BLOCKS] __attribute__((section(".fast_sums")));
static u32 data_blocks(void)
{
    u32 bytes = (u32)_data_end - (u32)_code_end, n = (bytes + DATA_BLOCK - 1) / DATA_BLOCK;
    return n < DATA_BLOCKS ? n : DATA_BLOCKS;
}
/* The sum of block `i`. The last block holds the rest of the data. */
static u32 data_sum(u32 i, u32 n)
{
    const u32 *p = _code_end + i * (DATA_BLOCK / 4);
    const u32 *end = i + 1 == n ? _data_end : p + DATA_BLOCK / 4;
    u32 sum = 0;
    while (p < end) { sum = *p++ + ((sum >> 27) | (sum << 5)); }
    return sum;
}
static void data_seal(void)
{
    u32 n = data_blocks();
    for (u32 i = 0; i < n; ++i) data_sums[i] = data_sum(i, n);
}
/* The .data of the file, before SCAN changed it. INSTALL changes .data too:
   relocate() moves the patch addresses in place, so a second try on the
   same .data would move them again. */
static u32 data_copy[DATA_COPY / 4] __attribute__((section(".fast_keep")));
static void data_keep(void)
{
    for (u32 *p = _code_end, *q = data_copy; p < _edata; ) *q++ = *p++;
}
static void data_reset(void)
{
    for (u32 *p = _code_end, *q = data_copy; p < _edata; ) *p++ = *q++;
    for (u32 *p = _edata; p < _keep_start; ) *p++ = 0;
}
/* Writes the first changed block and the number of changed blocks to the
   detail words, and returns 1, if a block changed. */
static int data_changed(u32 *array)
{
    u32 n = data_blocks(), changed = 0, first = 0;
    for (u32 i = 0; i < n; ++i)
        if (data_sum(i, n) != data_sums[i]) { if (!changed) first = (u32)(_code_end + i * (DATA_BLOCK / 4)); ++changed; }
    if (!changed) return 0;
    array[FAST_DETAIL_INDEX] = first;
    array[FAST_DETAIL_INDEX + 1] = changed;
    return 1;
}

/* The installer with one argument, as SCAN.SH and INSTALL.SH run it. The
   loader script reads the results from the card: PLAN.BIN after a scan,
   INSTALL.OK or INSTALL.ERR after an install. */
static int run(const char *mode)
{
    reset_files();
    char name[] = "CORE.BIN";
    char *argv[2] = {name, (char *)mode};
    int s = grkit_install_entry(2, argv);
    reset_files();
    return s;
}

/* The first call, from the head (head.S): scan the package on the card. */
static int scan(u32 *array)
{
    int s = run("--scan");
    struct ram_file *plan = ram_find(FAST_PLAN_FILE);
    if (!s && (!plan || plan->size < FAST_PLAN_WORDS * 4)) s = STATUS_PLAN_WRITE;
    if (!s) for (u32 i = 0; i < FAST_PLAN_WORDS; ++i) array[FAST_PLAN_INDEX + i] = ((const u32 *)plan->bytes)[i];
    RLOG(0, 0, 0, 0, "GR_MODULE_SCAN_EXIT_%d", s);
    array[FAST_STATUS_INDEX] = (u32)s;
    return s;
}

/* The later calls, from the install file of the release after the script
   allocated the planned arrays: install. INSTALL reads the plan that SCAN
   left in RAM. The loader calls it again after a not-ready status. */
static int install(u32 *array)
{
    int s = run("--install");
    RLOG(0, 0, 0, 0, "GR_MODULE_INSTALL_EXIT_%d", s);
    array[FAST_STATUS_INDEX] = (u32)s;
    return s;
}

/* The sums are taken and checked on the stack of the script task, after the
   call left the stack of this file. */
__attribute__((used)) int rtos_fastboot_scan(u32 *ba)
{
    data_keep();
    int s = on_stack(scan, ba, stack + STACK_SIZE);
    data_seal();
    return s;
}
__attribute__((used)) int rtos_fastboot_install(u32 *ba)
{
    if (data_changed(ba)) {
        RLOG(0, 0, 0, 0, "GRKit: the fast-boot data changed after SCAN");
        ba[FAST_STATUS_INDEX] = FAST_STATUS_DATA_CHANGED;
        return FAST_STATUS_DATA_CHANGED;
    }
    data_reset();
    int s = on_stack(install, ba, stack + STACK_SIZE);
    /* After the install, so it does not delay it. */
    if (!s) pack_save();
    data_seal();
    return s;
}
