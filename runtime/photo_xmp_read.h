#ifndef GRKIT_PHOTO_XMP_READ_H
#define GRKIT_PHOTO_XMP_READ_H
/* The reading of the XMP packet of a photo (runtime/photo_xmp.h): the
   settings that the packet names, the place and the value of a property,
   the pairs of grkit:State, and the packet of a DNG. The picture code reads
   a packet for a RAW development, and the menu code for the list of
   playback (runtime/playback_info.h). The includer defines u8, u32, and the
   header fields. */
typedef struct {
    u32 id, handle;
    const char *property;
} PhotoXmpSetting;
static const PhotoXmpSetting photo_xmp_settings[] = {GRKIT_PHOTO_XMP_SETTINGS};
/* The property of the state: "ID=value,..." with the state file ID and the
   value of each saved setting, in decimal. */
#define PHOTO_XMP_STATE "grkit:State"

/* Whether `name` is at `xmp + i`. */
static __attribute__((unused)) u32 photo_xmp_at(const char *xmp, u32 len, u32 i, const char *name, u32 m) {
    u32 k;
    if (i > len || m > len - i) return 0;
    for (k = 0; k < m; ++k)
        if (xmp[i + k] != name[k]) return 0;
    return 1;
}
/* Where property `name` is in `xmp` of `len` bytes, after a blank (an
   attribute) or `<` (an element): the place of the name, or 0. */
static __attribute__((unused)) u32 photo_xmp_where(const char *xmp, u32 len, const char *name) {
    u32 i, m = 0;
    while (name[m]) ++m;
    for (i = 1; i + m <= len; ++i)
        if (photo_xmp_at(xmp, len, i, name, m) &&
            (xmp[i - 1] == '<' || xmp[i - 1] == ' ' || xmp[i - 1] == '\t' || xmp[i - 1] == '\n' || xmp[i - 1] == '\r'))
            return i;
    return 0;
}
/* The value of property `name` in `xmp` of `len` bytes, as an attribute
   (name="..." after a blank) or as an element (<name>...</name>): its first
   byte, or 0, with its length in *n. A value that does not end in `xmp` is
   no value. */
static __attribute__((unused)) const char *photo_xmp_find(const char *xmp, u32 len, const char *name, u32 *n) {
    u32 i, k, m = 0;
    char close;
    while (name[m]) ++m;
    for (i = 1; i + m + 2 <= len; ++i) {
        if (!photo_xmp_at(xmp, len, i, name, m)) continue;
        if (xmp[i - 1] == '<' && xmp[i + m] == '>') {
            close = '<';
            k = i + m + 1;
        } else if ((xmp[i - 1] == ' ' || xmp[i - 1] == '\t' || xmp[i - 1] == '\n' || xmp[i - 1] == '\r') &&
                   xmp[i + m] == '=' && (xmp[i + m + 1] == '"' || xmp[i + m + 1] == '\'')) {
            close = xmp[i + m + 1];
            k = i + m + 2;
        } else {
            continue;
        }
        for (i = k; i < len && xmp[i] != close; ++i) {}
        if (i == len) return 0;
        *n = i - k;
        return xmp + k;
    }
    return 0;
}
/* The next "ID=value" pair of a list at *at: 1 with them, or 0 at the end. */
static __attribute__((unused)) u32 photo_xmp_next(const char *s, u32 n, u32 *at, u32 *id, u32 *value) {
    u32 i = *at, k = 0, v = 0, digits = 0;
    while (i < n && (s[i] == ' ' || s[i] == '\n' || s[i] == '\r' || s[i] == '\t')) ++i;
    if (i >= n) return 0;
    for (; i < n && s[i] >= '0' && s[i] <= '9'; ++i) k = k * 10 + (u32)(s[i] - '0');
    if (i < n && s[i] == '=')
        for (++i; i < n && s[i] >= '0' && s[i] <= '9'; ++i, ++digits) v = v * 10 + (u32)(s[i] - '0');
    while (i < n && s[i] != ',') ++i;
    *at = i + 1;
    *id = k;
    *value = v;
    return 1;
}
/* The XMP packet (tag 700) of IFD0 of the TIFF file whose first `len` bytes
   are in `buf`: its offset in the file, with its length in *n, or 0. The
   packet can be anywhere in the file: Lightroom moves it to the end when it
   saves metadata to the file. */
static __attribute__((unused)) u32 photo_xmp_tiff(const u8 *buf, u32 len, u32 *n) {
    u32 ifd, count, i;
    if (len < 8 || buf[0] != 'I' || buf[1] != 'I') return 0;
    ifd = buf[4] | buf[5] << 8 | buf[6] << 16 | (u32)buf[7] << 24;
    if (ifd > len - 2) return 0;
    count = buf[ifd] | buf[ifd + 1] << 8;
    for (i = 0; i < count && ifd + 2 + i * 12 + 12 <= len; ++i) {
        const u8 *e = buf + ifd + 2 + i * 12;
        if ((e[0] | e[1] << 8) != 700) continue;
        *n = e[4] | e[5] << 8 | e[6] << 16 | (u32)e[7] << 24;
        return e[8] | e[9] << 8 | e[10] << 16 | (u32)e[11] << 24;
    }
    return 0;
}
#endif
