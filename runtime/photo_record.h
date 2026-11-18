#ifndef GRKIT_PHOTO_RECORD_H
#define GRKIT_PHOTO_RECORD_H
/* The GRKit record of a photo: the settings of the shot in the XMP packet
   of the photo, in the namespace PHOTO_RECORD_NS. grkit:Core holds the core
   settings that a development copies (runtime/develop_settings.h) as
   "handle=value" with the handle in hex; grkit:Modules holds each module
   setting that the state file saves as "ID=value" with its state file ID in
   hex. Values are decimal. A RAW development starts from the record of its
   DNG (photo_record_apply), so a change of the settings after the shot does
   not change the development.

   The includer defines u8, u32, core_divide, the header fields, settings_table,
   setting_value, and runtime/develop_settings.h. */
#define PHOTO_RECORD_NS "https://grkit.app/ns/1.0/"
static const char photo_record_head[] =
    "<?xpacket begin=\"\xef\xbb\xbf\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>"
    "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">"
    "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">"
    "<rdf:Description rdf:about=\"\" xmlns:grkit=\"" PHOTO_RECORD_NS "\" grkit:Core=\"";
static const char photo_record_modules[] = "\" grkit:Modules=\"";
static const char photo_record_tail[] =
    "\"/></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";

/* Append `s` to `out` at `at`, within `size`: the new end, or size + 1
   when it does not fit. */
static __attribute__((unused)) u32 photo_record_put(char *out, u32 at, u32 size, const char *s) {
    while (*s) {
        if (at >= size) return size + 1;
        out[at++] = *s++;
    }
    return at;
}
static __attribute__((unused)) u32 photo_record_number(char *out, u32 at, u32 size, u32 value, u32 hex) {
    char digits[12];
    u32 n = 0, next, d;
    do {
        next = hex ? value >> 4 : core_divide(value, 10U);
        d = value - (hex ? next << 4 : next * 10U);
        digits[n++] = (char)(d < 10 ? '0' + d : 'a' + d - 10);
        value = next;
    } while (value && n < sizeof(digits));
    while (n) {
        if (at >= size) return size + 1;
        out[at++] = digits[--n];
    }
    return at;
}
static __attribute__((unused)) u32 photo_record_pair(char *out, u32 at, u32 size, u32 *first, u32 key, u32 value) {
    if (!*first) at = photo_record_put(out, at, size, ",");
    *first = 0;
    at = photo_record_number(out, at, size, key, 1);
    at = photo_record_put(out, at, size, "=");
    return photo_record_number(out, at, size, value, 0);
}
/* The packet of the current settings in `out`: its length, or 0 when it
   does not fit in `size` bytes. */
static __attribute__((unused)) u32 photo_record_build(volatile u32 *h, char *out, u32 size) {
    static const u16 core[] = {GRKIT_CORE_GRAIN, GRKIT_CORE_GRAIN_SIZE, GRKIT_CORE_BORDER, GRKIT_CORE_BORDER_COLOR};
    volatile u32 *t = settings_table(h);
    u32 at = photo_record_put(out, 0, size, photo_record_head), first = 1, i;
    for (i = 0; i < sizeof(core) / sizeof(core[0]); ++i)
        at = photo_record_pair(out, at, size, &first, core[i], setting_value(h, core[i]));
    at = photo_record_put(out, at, size, photo_record_modules);
    first = 1;
    for (i = 0; t && i < t[0]; ++i)
        if (t[SETTINGS_ENTRY(i)]) at = photo_record_pair(out, at, size, &first, t[SETTINGS_ENTRY(i)], setting_value(h, i));
    at = photo_record_put(out, at, size, photo_record_tail);
    return at <= size ? at : 0;
}

/* The value of attribute `name` in `xmp` of `len` bytes: its first byte, or
   0, with its length in *n. */
static __attribute__((unused)) const char *photo_record_find(const char *xmp, u32 len, const char *name, u32 *n) {
    u32 i, k, m = 0;
    while (name[m]) ++m;
    for (i = 0; i + m + 2 <= len; ++i) {
        for (k = 0; k < m && xmp[i + k] == name[k]; ++k) {}
        if (k < m || xmp[i + m] != '=' || xmp[i + m + 1] != '"') continue;
        i += m + 2;
        for (k = i; k < len && xmp[k] != '"'; ++k) {}
        *n = k - i;
        return xmp + i;
    }
    return 0;
}
/* The next "key=value" pair of a list at *at: 1 with them, or 0 at the end. */
static __attribute__((unused)) u32 photo_record_next(const char *s, u32 n, u32 *at, u32 *key, u32 *value) {
    u32 i = *at, k = 0, v = 0, digits = 0;
    if (i >= n) return 0;
    for (; i < n && s[i] != '='; ++i) {
        char c = s[i];
        k = k << 4 | (u32)(c >= 'a' ? c - 'a' + 10 : c >= 'A' ? c - 'A' + 10 : c - '0');
    }
    for (++i; i < n && s[i] != ','; ++i, ++digits) v = v * 10 + (u32)(s[i] - '0');
    *at = i + 1;
    *key = k;
    *value = v;
    return digits != 0;
}
/* The record in `xmp` into the copy of a development that starts. A value
   that the setting does not offer is left out. */
static __attribute__((unused)) void photo_record_apply(volatile u32 *h, const char *xmp, u32 len) {
    volatile u32 *t = settings_table(h), *c;
    const char *s;
    u32 n, at, key, value, i;
    if (!h[GRKIT_HOST_SERVICES / 4]) return;
    c = services_word(h, SERVICES_DEVELOP);
    if ((s = photo_record_find(xmp, len, "grkit:Core", &n)) != 0)
        for (at = 0; photo_record_next(s, n, &at, &key, &value);) {
            if (!IS_CORE_SETTING(key) || value >= core_setting_values[CORE_SETTING(key)]) continue;
            for (i = 0; i < DEVELOP_COPY_HEADER_WORDS; ++i)
                if (develop_copy_words[i] == core_setting_words[CORE_SETTING(key)]) c[DEVELOP_COPY_HEADER / 4 + i] = value;
        }
    if (!t || (s = photo_record_find(xmp, len, "grkit:Modules", &n)) == 0) return;
    for (at = 0; photo_record_next(s, n, &at, &key, &value);)
        for (i = 0; key && i < t[0] && i < GRKIT_MODULE_SETTINGS_MAX; ++i)
            if (t[SETTINGS_ENTRY(i)] == key) {
                if (value < t[SETTINGS_ENTRY(i) + 2]) c[DEVELOP_COPY_MODULE / 4 + i] = value;
                break;
            }
}

/* The XMP packet (tag 700) of IFD0 of the TIFF file in `buf`, of which
   `len` bytes are read: its offset, with its length in *n, or 0. */
static __attribute__((unused)) u32 photo_record_tiff_xmp(const u8 *buf, u32 len, u32 *n) {
    u32 ifd, count, i, off;
    if (len < 8 || buf[0] != 'I' || buf[1] != 'I') return 0;
    ifd = buf[4] | buf[5] << 8 | buf[6] << 16 | (u32)buf[7] << 24;
    if (ifd > len - 2) return 0;
    count = buf[ifd] | buf[ifd + 1] << 8;
    for (i = 0; i < count && ifd + 2 + i * 12 + 12 <= len; ++i) {
        const u8 *e = buf + ifd + 2 + i * 12;
        if ((e[0] | e[1] << 8) != 700) continue;
        *n = e[4] | e[5] << 8 | e[6] << 16 | (u32)e[7] << 24;
        off = e[8] | e[9] << 8 | e[10] << 16 | (u32)e[11] << 24;
        return off && off <= len && *n <= len - off ? off : 0;
    }
    return 0;
}
#endif
