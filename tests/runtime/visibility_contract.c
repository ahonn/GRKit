#include <stdio.h>
#include <stdlib.h>
typedef unsigned char u8;
typedef unsigned int u32;
#include "visibility.h"

/* Parses the hex-encoded MODULES.TXT in argv[1]. Prints the hidden mask, or
   "invalid", as the installer would decide. */
int main(int argc, char **argv)
{
    u8 text[VISIBILITY_FILE_MAX + 1];
    u32 n = 0, hidden = 0;
    const char *hex = argc == 2 ? argv[1] : "";
    while (hex[n * 2] && hex[n * 2 + 1] && n < sizeof(text)) {
        char pair[3] = {hex[n * 2], hex[n * 2 + 1], 0};
        text[n++] = (u8)strtoul(pair, 0, 16);
    }
    if (n > VISIBILITY_FILE_MAX || !visibility_parse(text, n, &hidden)) puts("invalid");
    else printf("%u\n", hidden);
    return 0;
}
