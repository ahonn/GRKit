#ifndef GRKIT_MENU_LABELS_H
#define GRKIT_MENU_LABELS_H

/* Keep the module summary in its effect record for deferred native drawing. */
static void module_summary(volatile unsigned short *out, const unsigned char *name)
{
    static const char prefix[] = "Image Control: ";
    unsigned int i, j;
    for (i = 0; i < sizeof(prefix) - 1; ++i) out[i] = prefix[i];
    for (j = 0; j < 16 && name[j]; ++j) out[i++] = name[j];
    out[i] = 0;
}

#endif
