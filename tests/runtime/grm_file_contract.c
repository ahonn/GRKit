/* Reads a module file that grkit_core::grm wrote, links it as the installer
   does (runtime/module_link.h), and prints what it read: the header fields,
   then the linked code and data words. The Rust test compares them with the
   module that it encoded. Usage: grm_file_contract FILE */
#include <stdio.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned int u32;
#include "module_link.h"

#define CODE_BASE 0x54000000U
#define DATA_BASE 0x59000000U

static u32 fnv(const char *s)
{
    u32 h = 0x811c9dc5;
    while (*s) { h ^= (u8)*s++; h *= 0x01000193; }
    return h;
}

int main(int argc, char **argv)
{
    static u8 file[GRM_HEADER + 4 * GRM_MAX_SEGMENT], code[GRM_MAX_SEGMENT], data[GRM_MAX_SEGMENT];
    struct grm_module m;
    struct grm_setting settings[GRM_SETTINGS];
    char uses[GRM_USES][GRM_NAME + 1];
    u32 setting_count, use_count, failed = 0;
    /* The imports that the Rust test names. */
    const struct grm_symbol symbols[] = {{fnv("overlay_label"), 0x11111111}, {fnv("divide"), 0x22222222}};
    if (argc != 2) return 2;
    FILE *in = fopen(argv[1], "rb");
    if (!in) return 2;
    u32 size = (u32)fread(file, 1, sizeof file, in);
    fclose(in);
    int status = grm_parse(file, size, &m);
    if (!status) status = grm_link(&m, code, data, CODE_BASE, DATA_BASE, symbols, 2, &failed);
    if (!status) status = grm_settings(&m, code, CODE_BASE, settings, &setting_count, uses, &use_count);
    printf("status %d\n", status);
    if (status) return 1;
    printf("id %s\nversion %u\norder %u\ncameras %u\nabi %u\n", m.id, m.version, grm_word(file + GRM_ORDER), m.cameras,
           m.abi);
    printf("code %u\ndata %u\nbss %u\ndescriptor %u\n", m.code_size, m.data_size, m.bss_size, m.descriptor);
    printf("settings %u\nuses %u\n", setting_count, use_count);
    for (u32 i = 0; i < setting_count; ++i) printf("setting %s %u %u\n", settings[i].key, settings[i].values, settings[i].flags);
    for (u32 i = 0; i < use_count; ++i) printf("use %s\n", uses[i]);
    for (u32 i = 0; i < m.code_size; i += 4) printf("c %08x\n", grm_word(code + i));
    for (u32 i = 0; i < m.data_size; i += 4) printf("d %08x\n", grm_word(data + i));
    return 0;
}
