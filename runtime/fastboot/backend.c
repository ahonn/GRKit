/* The RTOS side of the installer: the backend of installer.c built with
 * -DRTOS_NATIVE, which the head of a fast-boot file (head.S) loads and calls.
 *
 * installer.c performs all I/O through syscall6
 * and all camera-memory access through at()/word()/store(), which resolve
 * against the address that mmap returns. This backend:
 *   - serves the card paths (/media/storage1/...) from the SD card through the
 *     RTOS file API (C:\...), so every file the installer writes reaches the
 *     card as on Linux;
 *   - keeps no file in memory: the head loads the whole file again for
 *     INSTALL, so nothing lasts from SCAN to INSTALL but the card;
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

static u32 slen(const char *s) { u32 n = 0; while (s[n]) ++n; return n; }
static int seq(const char *a, const char *b) { while (*a && *a == *b) { ++a; ++b; } return *a == *b; }
static int prefix(const char *s, const char *p) { while (*p) if (*s++ != *p++) return 0; return 1; }

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

/* ---- directory listings --------------------------------------------------- */
#define LIST_NAMES 256U
#define NAME_SIZE 16U    /* 8.3 names */
struct listing { u32 count; char names[LIST_NAMES][NAME_SIZE]; };
#define LISTINGS 4U
static struct listing listings[LISTINGS];
static int listing_used[LISTINGS];
static u8 dir_entry[2048] __attribute__((aligned(8)));

static int list_directory(const char *dos, struct listing *l)
{
    u32 dno = 0;
    if (FS_OPENDIR(dos, &dno)) return -1;
    l->count = 0;
    for (;;) {
        for (u32 i = 0; i < sizeof dir_entry; ++i) dir_entry[i] = 0;
        if (FS_READDIR(dno, dir_entry)) break;
        if (l->count >= LIST_NAMES) continue;
        u32 n = 0;
        while (n + 1 < NAME_SIZE && dir_entry[n]) { l->names[l->count][n] = (char)dir_entry[n]; ++n; }
        l->names[l->count][n] = 0;
        ++l->count;
    }
    FS_CLOSEDIR(dno);
    return 0;
}

/* ---- open-file table ------------------------------------------------------ */
enum kind { FREE, CARD_FILE, CARD_DIR, MEMORY };
struct open_file { enum kind kind; u32 fno; u32 size; u32 position; int writing; struct listing *list; };
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
    } else if (dos_path(path, dos)) {
        int exists = 0;
        if (flags == 0xc000) {
            u32 k = 0;
            while (k < LISTINGS && listing_used[k]) ++k;
            if (k == LISTINGS) return EMFILE_ERROR;
            if (list_directory(dos, &listings[k])) return ENOENT_ERROR;
            listing_used[k] = 1;
            o->list = &listings[k];
            o->kind = CARD_DIR;
        } else if (flags == 0x8800) {
            o->size = card_size(dos, &exists);
            if (!exists) return ENOENT_ERROR;
            int f = FS_OPEN(dos, MODE_RDWR, 0);
            if (f < 0) return ENOENT_ERROR;
            o->fno = (u32)f;
            o->kind = CARD_FILE;
        } else if (flags == 0xc1 || flags == 0x241) {
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
        if (o->kind != CARD_FILE || o->writing) return EBADF_ERROR;
        { u32 left = o->size - o->position, k = left < c ? left : c, len = 0;
          /* The RTOS read reports the requested count, so read exactly what is left. */
          if (k && FS_READ(o->fno, (void *)b, k, &len)) return EIO_ERROR;
          o->position += k; return (long)k; }
    case 4: /* write: fd 1 = stdout */
        if (a == 1 || a == 2) { putbytes((const char *)b, c); return (long)c; }
        o = opened(a);
        if (!o || !o->writing) return EBADF_ERROR;
        { u32 len = 0;
          if (c && FS_WRITE(o->fno, (void *)b, c, &len)) return EIO_ERROR;
          o->position += c; return (long)c; }
    case 5: /* open */
        return do_open((const char *)a, b);
    case 6: /* close */
        o = opened(a); if (!o) return EBADF_ERROR;
        if (o->kind == CARD_FILE) FS_CLOSE(o->fno);
        if (o->kind == CARD_DIR) for (u32 k = 0; k < LISTINGS; ++k) if (&listings[k] == o->list) listing_used[k] = 0;
        o->kind = FREE;
        return 0;
    case 10: /* unlink */
        if (!dos_path((const char *)a, dos)) return ENOENT_ERROR;
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
        FS_REMOVE(dos2);
        return FS_RENAME(dos, dos2) ? ENOENT_ERROR : 0;
    case 39: /* mkdir */
        if (!dos_path((const char *)a, dos)) return ENOENT_ERROR;
        return FS_MKDIR(dos) ? EIO_ERROR : 0;
    case 91: /* munmap */
        return 0;
    case 192: /* mmap: physical identity map so at(p) == (u8*)p */
        return (long)MAP_BASE;
    case 0xf0002: /* msync alias -> real cache maintenance over [a, b) */
        cache_sync(a, b - a);
        return 0;
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
    RLOG(0, 0, 0, 0, "GR_MODULE_SCAN_EXIT_%d", s);
    array[FAST_STATUS_INDEX] = (u32)s;
    return s;
}

/* The later calls, from the install file of the release after the script
   allocated the planned arrays: install. The head loaded this file again
   and cleared the BSS, so INSTALL reads its plan from the card. */
static int install(u32 *array)
{
    int s = run("--install");
    RLOG(0, 0, 0, 0, "GR_MODULE_INSTALL_EXIT_%d", s);
    array[FAST_STATUS_INDEX] = (u32)s;
    return s;
}

__attribute__((used)) int rtos_fastboot_scan(u32 *ba) { return on_stack(scan, ba, stack + STACK_SIZE); }
__attribute__((used)) int rtos_fastboot_install(u32 *ba) { return on_stack(install, ba, stack + STACK_SIZE); }
