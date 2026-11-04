/* Links a built module file as the installer does (runtime/module_link.h),
   with each import at a stand-in address, and checks its registrations: its
   pages, layers, filters, and indicators. Prints the status, 0 when the
   installer would load the module. Usage: grm_registrations_contract FILE */
#include <stdio.h>

typedef unsigned char u8;
typedef unsigned int u32;
#include "module_link.h"

#define CODE_BASE 0x54000000U
#define DATA_BASE 0x59000000U

int main(int argc, char **argv)
{
    static u8 file[GRM_HEADER + 4 * GRM_MAX_SEGMENT], code[GRM_MAX_SEGMENT], data[GRM_MAX_SEGMENT];
    static struct grm_symbol symbols[GRM_MAX_SEGMENT / 8];
    struct grm_module m;
    struct grm_setting settings[GRM_SETTINGS];
    char uses[GRM_USES][GRM_NAME + 1];
    struct grm_layer layers[16];
    struct grm_page pages[16];
    struct grm_filter filters[16];
    u32 setting_count, use_count, layer_count = 0, page_count = 0, filter_count = 0, failed = 0;
    if (argc != 2) return 2;
    FILE *in = fopen(argv[1], "rb");
    if (!in) return 2;
    u32 size = (u32)fread(file, 1, sizeof file, in);
    fclose(in);
    int status = grm_parse(file, size, &m);
    /* The core exports each import of a module that grkit module build
       accepts. */
    for (u32 i = 0; !status && i < m.imports && i < sizeof symbols / sizeof symbols[0]; ++i) {
        symbols[i].hash = grm_word(m.import_entries + i * 8);
        symbols[i].value = 0x60000000U + i * 4;
    }
    if (!status) status = grm_link(&m, code, data, CODE_BASE, DATA_BASE, symbols, m.imports, &failed);
    if (!status) status = grm_settings(&m, code, CODE_BASE, settings, &setting_count, uses, &use_count);
    if (!status)
        status = grm_registrations(&m, code, CODE_BASE, settings, setting_count, use_count, layers, &layer_count,
                                   pages, &page_count, filters, &filter_count, 16);
    printf("status %d\n", status);
    return status ? 1 : 0;
}
