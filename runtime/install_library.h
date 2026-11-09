#ifndef GRKIT_INSTALL_LIBRARY_H
#define GRKIT_INSTALL_LIBRARY_H
/* A part of the installer: the package list and the effects library.
   installer.c includes it once, in its order, after the helpers and the state
   that this part uses.

   PACKAGE.TXT names the files of the package, separated by ASCII whitespace:
   its effects in the inbox and its modules. The effects of the package
   install before the other effects of the library. A package module loads
   only if the list names it, so a merge copy of a new package that leaves
   an old module behind does not load it. Without the list, each package
   module loads. */
#define NAMES_MAX 64U
#define PACKAGE_FILE_MAX 1024U
/* ASCII whitespace, which separates the names of the list. */
static int text_space(u8 c) { return c==' ' || c=='\t' || c=='\r' || c=='\n'; }
static char package_effects[NAMES_MAX][CATALOG_NAME_SIZE], package_modules[NAMES_MAX][CATALOG_NAME_SIZE];
static u32 package_effect_count, package_module_count;
static int package_listed;
static int same_name(const char *a, const char *b)
{
    for(;;) {
        char x=*a++, y=*b++;
        if(x>='a' && x<='z') x-=32;
        if(y>='a' && y<='z') y-=32;
        if(x!=y) return 0;
        if(!x) return 1;
    }
}
/* A module file name: <ID>.GRM, 5 to 12 characters. */
static int module_file_name(const char *name, u32 n)
{
    if (n < 5 || n > 12 || name[n - 4] != '.') return 0;
    for (u32 i = 0; i < 3; ++i) if ((name[n - 3 + i] & ~0x20) != "GRM"[i]) return 0;
    return 1;
}
static void read_package_list(void)
{
    static u8 text[PACKAGE_FILE_MAX];
    package_effect_count = package_module_count = 0;
    long have = read_text(package_path, text, sizeof(text));
    package_listed = have >= 0;
    for (long i = 0; i < have;) {
        while (i < have && text_space(text[i])) ++i;
        long start = i;
        while (i < have && !text_space(text[i])) ++i;
        u32 n = (u32)(i - start);
        const char *name = (const char *)text + start;
        if (!n || n >= CATALOG_NAME_SIZE) continue;
        if (catalog_name(name, n) && package_effect_count < NAMES_MAX) {
            copy((u8 *)package_effects[package_effect_count], text + start, n);
            package_effects[package_effect_count++][n] = 0;
        } else if (module_file_name(name, n) && package_module_count < NAMES_MAX) {
            copy((u8 *)package_modules[package_module_count], text + start, n);
            package_modules[package_module_count++][n] = 0;
        }
    }
}
static int package_effect(const char *name)
{
    for (u32 i = 0; i < package_effect_count; ++i) if (same_name(package_effects[i], name)) return 1;
    return 0;
}
static int package_module(const char *name)
{
    if (!package_listed) return 1;
    for (u32 i = 0; i < package_module_count; ++i) if (same_name(package_modules[i], name)) return 1;
    return 0;
}
/* The package effects that `collect` could not copy into the library at
   this power-on. They install from the inbox. */
static char pending[NAMES_MAX][CATALOG_NAME_SIZE];
static u32 pending_count;
static u8 existing[GRKIT_GRF_MAX_SIZE];
/* The names of the effect files to copy from one folder. */
struct effect_names {
    char name[NAMES_MAX][CATALOG_NAME_SIZE];
    u32 count;
};
static struct effect_names copies;
static void effect_name(const char *name, u32 n, void *context)
{
    struct effect_names *list = context;
    if (catalog_name(name, n) && list->count < NAMES_MAX) catalog_copy_name(list->name[list->count++], name);
}
/* List the effect files of `folder` in `copies`: 0, or nonzero when the
   folder does not open or does not list. */
static int list_effects(const char *folder)
{
    copies.count = 0;
    long fd = syscall6(5, (u32)folder, 0xc000, 0, 0, 0, 0);
    if (fd < 0) return 1;
    static const struct directory_calls calls = {directory_read, directory_seek};
    int status = directory_list(fd, directory_entries, sizeof(directory_entries), &calls, effect_name, &copies);
    syscall6(6, fd, 0, 0, 0, 0, 0);
    return status;
}
/* IMPORTED.TXT records the package that the library has: for each listed
   inbox file, its name and the FNV-1a hash of its bytes. On the RTOS side a
   line also has the size of the file and the stamp of its times
   (FAST_FILE_STAMP): while both stay the same, a start takes the hash from
   the line and does not read the file. */
#define IMPORTED_MAX (NAMES_MAX * 44U)
static u8 record[IMPORTED_MAX], recorded[IMPORTED_MAX];
static u32 record_hex(u32 n, u32 value)
{
    record[n++] = ' ';
    for (int shift = 28; shift >= 0; shift -= 4) record[n++] = (u8)"0123456789ABCDEF"[(value >> shift) & 15];
    return n;
}
static u32 record_line(u32 n, const char *name, u32 hash, int stamped, u32 size, u32 stamp)
{
    u32 k = length(name);
    if (n + k + 29 > IMPORTED_MAX) return n;
    copy(record + n, (const u8 *)name, k); n += k;
    n = record_hex(n, hash);
    if (stamped) { n = record_hex(n, size); n = record_hex(n, stamp); }
    record[n++] = '\r'; record[n++] = '\n';
    return n;
}
/* The words of a line of IMPORTED.TXT after the name: the hash, then the
   size and the stamp. Returns the number of words. */
static u32 recorded_words(const u8 *line, u32 n, u32 *words)
{
    u32 count = 0, i = 0;
    while (i < n && !text_space(line[i])) ++i;
    while (count < 3) {
        while (i < n && (line[i] == ' ' || line[i] == '\t')) ++i;
        u32 value = 0, digits = 0;
        for (; i < n && digits < 8; ++i, ++digits) {
            u8 c = line[i];
            u32 d = c >= '0' && c <= '9' ? (u32)(c - '0') : c >= 'A' && c <= 'F' ? (u32)(c - 'A' + 10) : 16U;
            if (d > 15) break;
            value = value << 4 | d;
        }
        if (digits != 8) break;
        words[count++] = value;
    }
    return count;
}
/* The recorded hash of `name`, if its line has the same size and stamp. */
static int recorded_hash(long had, const char *name, u32 size, u32 stamp, u32 *hash)
{
    for (long i = 0; i < had;) {
        long start = i;
        while (i < had && recorded[i] != '\n') ++i;
        const u8 *line = recorded + start;
        u32 n = (u32)(i - start), k = length(name), words[3];
        if (i < had) ++i;
        if (n <= k || !text_space(line[k])) continue;
        char head[CATALOG_NAME_SIZE];
        if (k >= sizeof(head)) continue;
        copy((u8 *)head, line, k);
        head[k] = 0;
        if (!same_name(head, name)) continue;
        if (recorded_words(line, n, words) == 3 && words[1] == size && words[2] == stamp) {
            *hash = words[0];
            return 1;
        }
        return 0;
    }
    return 0;
}
/* The size and the stamp of a card file without reading it, on the RTOS
   side only. */
static int file_stamp(const char *path, u32 *size, u32 *stamp)
{
#ifdef RTOS_NATIVE
    u32 out[2];
    if (syscall6(FAST_FILE_STAMP, (u32)path, (u32)out, 0, 0, 0, 0)) return 0;
    *size = out[0];
    *stamp = out[1];
    return 1;
#else
    (void)path; (void)size; (void)stamp;
    return 0;
#endif
}
static u32 fnv(const u8 *data, u32 size)
{
    u32 hash = 0x811c9dc5;
    for (u32 i = 0; i < size; ++i) { hash ^= data[i]; hash *= 0x01000193; }
    return hash;
}
/* Copy the effects of a new package into the library, where each replaces
   the file with the same name. The package is new when the record of its
   listed files differs from IMPORTED.TXT. The first power-on without the
   library copies every valid inbox file. The installer deletes no file and
   renames none: on the camera's Linux, a rename does not replace a file (on
   GR IV exFAT it fails with an I/O error), and on GR IV exFAT a delete made
   later lookups fail, also the request that runs INSTALL. So the inbox keeps
   its files, and a copy of the same package does not bring back an effect
   that a person deleted from the library. If the power fails during a copy,
   IMPORTED.TXT stays old, and a later power-on copies again. */
static void copy_effects(const char *folder, int package)
{
    for (u32 i = 0; i < copies.count; ++i) {
        char from[EFFECT_PATH_SIZE], to[EFFECT_PATH_SIZE];
        if (!join(from, sizeof(from), folder, copies.name[i]) || !join(to, sizeof(to), library_path, copies.name[i])) continue;
        u32 size = read_effect(from, temporary);
        if (!size || !valid_profile(temporary, size)) continue;
        if (read_effect(to, existing) == size && equal(existing, temporary, size)) continue;
        if (!write_file(to, temporary, size) && package && package_effect(copies.name[i]) && pending_count < NAMES_MAX)
            catalog_copy_name(pending[pending_count++], copies.name[i]);
    }
}
static void collect(void)
{
    pending_count = 0;
    long dir = syscall6(5, (u32)library_path, 0xc000, 0, 0, 0, 0);
    int first = dir == -2;
    if (dir >= 0) syscall6(6, dir, 0, 0, 0, 0, 0);
    else if (!first) return;
    u32 n = 0;
    long had = read_text(imported_path, recorded, sizeof(recorded));
    for (u32 i = 0; i < package_effect_count; ++i) {
        char from[EFFECT_PATH_SIZE];
        if (!join(from, sizeof(from), inbox_path, package_effects[i])) continue;
        u32 listed = 0, stamp = 0, hash = 0;
        int stamped = file_stamp(from, &listed, &stamp);
        if (stamped && recorded_hash(had, package_effects[i], listed, stamp, &hash)) {
            n = record_line(n, package_effects[i], hash, 1, listed, stamp);
            continue;
        }
        u32 size = read_effect(from, temporary);
        if (size && valid_profile(temporary, size))
            n = record_line(n, package_effects[i], fnv(temporary, size), stamped && size == listed, listed, stamp);
    }
    if (!first && had == (long)n && equal(recorded, record, n)) return;
    if (first) {
        long fd = syscall6(5, (u32)inbox_path, 0xc000, 0, 0, 0, 0);
        if (fd < 0) return;
        syscall6(6, fd, 0, 0, 0, 0, 0);
    }
    long made = syscall6(39, (u32)data_path, 0755, 0, 0, 0, 0);
    if (made < 0 && made != -17) return;
    made = syscall6(39, (u32)library_path, 0755, 0, 0, 0, 0);
    if (made < 0 && made != -17) return;
    if (first) {
        /* The effects of GRKit 1.2 come first, so a package effect with the
           same name replaces an old one. */
        if (!list_effects(legacy_effects_path)) copy_effects(legacy_effects_path, 0);
        if (list_effects(inbox_path)) return;
    } else {
        copies.count = 0;
        for (u32 i = 0; i < package_effect_count; ++i) catalog_copy_name(copies.name[copies.count++], package_effects[i]);
    }
    copy_effects(inbox_path, 1);
    if (!pending_count) write_file(imported_path, record, n);
}
#endif
