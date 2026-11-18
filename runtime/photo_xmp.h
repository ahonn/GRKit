#ifndef GRKIT_PHOTO_XMP_H
#define GRKIT_PHOTO_XMP_H
/* The XMP packet of a photo (grkit_core::photo_xmp). A JPEG has it in an
   APP1 segment (runtime/xmp.h), a DNG in tag 700 of IFD0. In the namespace
   GRKIT_PHOTO_XMP_NAMESPACE, prefix grkit:
   - grkit:FilmLook, the name of the look, which is also a dc:subject
     keyword: Lightroom shows keywords, not other namespaces.
   - A property for each setting that changes the picture
     (GRKIT_PHOTO_XMP_SETTINGS), with the English menu text of its value.
   - grkit:State, "ID=value,..." with the state file ID and the value of each
     saved setting, in decimal. A RAW development starts from it
     (photo_xmp_apply). Names and texts can change; the IDs do not.

   The includer defines u8, u16, u32, core_divide, the header fields, WORD,
   settings_table, setting_value, runtime/develop_settings.h, runtime/crop.h,
   and runtime/xmp.h (xmp_put_name). */

typedef struct {
    u32 id, handle;
    const char *property;
} PhotoXmpSetting;
static const PhotoXmpSetting photo_xmp_settings[] = {GRKIT_PHOTO_XMP_SETTINGS};
static const u32 photo_xmp_core_ids[] = {GRKIT_CORE_SETTING_IDS};
/* English, the first menu language. */
#define PHOTO_XMP_LANGUAGE 0U
#define PHOTO_XMP_NO_SETTING 0xffffffffU
/* A menu text at an address. A host test maps it. */
#ifndef PHOTO_XMP_TEXT
#define PHOTO_XMP_TEXT(address) ((const volatile u16 *)(address))
#endif

static const char photo_xmp_head[] =
    "<?xpacket begin=\"\xef\xbb\xbf\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>"
    "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">"
    "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">"
    "<rdf:Description rdf:about=\"\" xmlns:grkit=\"" GRKIT_PHOTO_XMP_NAMESPACE "\"";
static const char photo_xmp_tail[] = "</rdf:RDF></x:xmpmeta>";

/* Each writer appends to `out` at `at`, within `size`: the new end, or
   size + 1 when it does not fit. */
static __attribute__((unused)) u32 photo_xmp_put(u8 *out, u32 at, u32 size, const char *s) {
    while (*s) {
        if (at >= size) return size + 1;
        out[at++] = (u8)*s++;
    }
    return at;
}
static __attribute__((unused)) u32 photo_xmp_number(u8 *out, u32 at, u32 size, u32 value) {
    char digits[12];
    u32 n = 0, next;
    do {
        next = core_divide(value, 10U);
        digits[n++] = (char)('0' + value - next * 10U);
        value = next;
    } while (value && n < sizeof(digits));
    while (n) {
        if (at >= size) return size + 1;
        out[at++] = (u8)digits[--n];
    }
    return at;
}
/* A UTF-16 text of at most `max` units, as escaped UTF-8. */
static __attribute__((unused)) u32 photo_xmp_text(u8 *out, u32 at, u32 size, const volatile u16 *text, u32 max) {
    u32 n = xmp_put_name(0, text, max);
    if (at > size || n > size - at) return size + 1;
    xmp_put_name(out + at, text, max);
    return at + n;
}

/* The handle of a setting of the list: its core handle, or the module
   setting with its state file ID, or PHOTO_XMP_NO_SETTING without that
   module. */
static __attribute__((unused)) u32 photo_xmp_handle(volatile u32 *h, const PhotoXmpSetting *s) {
    volatile u32 *t = settings_table(h);
    u32 i;
    if (s->handle) return s->handle;
    for (i = 0; t && i < t[0]; ++i)
        if (t[SETTINGS_ENTRY(i)] == s->id) return i;
    return PHOTO_XMP_NO_SETTING;
}
/* The English menu text of value `value` of a setting: the rows of the
   module page of the setting, or its row on a page (a child page of a group
   has its setting as a row). 0 without one. */
static __attribute__((unused)) const volatile u16 *photo_xmp_label(volatile u32 *h, u32 handle, u32 value) {
    u32 n = h[GRKIT_HOST_MODULE_PAGE_COUNT / 4], i, page, table = 0, rows, list, r;
    if (n > GRKIT_MODULE_PAGES_MAX) return 0;
    for (i = 0; i < n && !table; ++i) {
        page = h[GRKIT_HOST_MODULE_PAGES / 4 + i * 3];
        if (!page) continue;
        if (h[GRKIT_HOST_MODULE_PAGES / 4 + i * 3 + 1] == handle && (table = WORD(page, GRM_PAGE_LABELS_AT))) break;
        if (WORD(page, 0) < GRM_PAGE_SETTINGS_AT + 4U) continue;
        rows = WORD(page, GRM_PAGE_SETTING_ROWS_AT);
        list = WORD(page, GRM_PAGE_SETTINGS_AT);
        for (r = 0; list && r < rows && !table; ++r)
            if (WORD(list + r * GRM_ROW, GRM_ROW_SETTING_AT) == handle) table = WORD(list + r * GRM_ROW, GRM_ROW_VALUES_AT);
    }
    return table ? PHOTO_XMP_TEXT(WORD(table, (value * GRKIT_TEXT_LANGUAGES + PHOTO_XMP_LANGUAGE) * 4)) : 0;
}
/* The longest menu text that a value takes, in UTF-16 units. */
#define PHOTO_XMP_LABEL_UNITS 64U
/* ` grkit:Name="text"` of each setting that changes the picture. Framing
   and Crop name a crop, so their frame-only modes leave them out. A value
   without a menu text is a number. */
static __attribute__((unused)) u32 photo_xmp_properties(volatile u32 *h, u8 *out, u32 at, u32 size) {
    const volatile u16 *label;
    u32 i, handle, value;
    for (i = 0; i < sizeof(photo_xmp_settings) / sizeof(photo_xmp_settings[0]); ++i) {
        handle = photo_xmp_handle(h, &photo_xmp_settings[i]);
        if (handle == PHOTO_XMP_NO_SETTING) continue;
        if (handle == GRKIT_CORE_ASPECT && setting_value(h, GRKIT_CORE_ASPECT_MODE)) continue;
        if (handle == GRKIT_CORE_CROP && setting_value(h, GRKIT_CORE_CROP_MODE)) continue;
        value = setting_value(h, handle);
        at = photo_xmp_put(out, at, size, " grkit:");
        at = photo_xmp_put(out, at, size, photo_xmp_settings[i].property);
        at = photo_xmp_put(out, at, size, "=\"");
        label = handle == GRKIT_CORE_CROP ? crop_lens_text(value) : 0;
        if (!label) label = photo_xmp_label(h, handle, value);
        at = label ? photo_xmp_text(out, at, size, label, PHOTO_XMP_LABEL_UNITS) : photo_xmp_number(out, at, size, value);
        at = photo_xmp_put(out, at, size, "\"");
    }
    return at;
}
static __attribute__((unused)) u32 photo_xmp_pair(u8 *out, u32 at, u32 size, u32 *first, u32 id, u32 value) {
    if (!*first) at = photo_xmp_put(out, at, size, ",");
    *first = 0;
    at = photo_xmp_number(out, at, size, id);
    at = photo_xmp_put(out, at, size, "=");
    return photo_xmp_number(out, at, size, value);
}
/* ` grkit:State="..."`: the core settings that have a state file ID, then
   each saved module setting. */
static __attribute__((unused)) u32 photo_xmp_state(volatile u32 *h, u8 *out, u32 at, u32 size) {
    volatile u32 *t = settings_table(h);
    u32 first = 1, i;
    at = photo_xmp_put(out, at, size, " grkit:State=\"");
    for (i = 0; i < GRKIT_CORE_SETTING_COUNT; ++i)
        if (photo_xmp_core_ids[i])
            at = photo_xmp_pair(out, at, size, &first, photo_xmp_core_ids[i], setting_value(h, GRKIT_CORE_SETTING_BASE + i));
    for (i = 0; t && i < t[0]; ++i)
        if (t[SETTINGS_ENTRY(i)]) at = photo_xmp_pair(out, at, size, &first, t[SETTINGS_ENTRY(i)], setting_value(h, i));
    return photo_xmp_put(out, at, size, "\"");
}
/* The packet in `out`, without its end (xmp_end): its length, or 0
   when it does not fit in `size` bytes or has nothing. `look` is the name of
   the look (`max` UTF-16 units), or 0; `settings` adds the settings. */
static __attribute__((unused)) u32 photo_xmp_build(volatile u32 *h, u8 *out, u32 size, const volatile u16 *look, u32 max,
                                                   u32 settings) {
    u32 at;
    if (look && !look[0]) look = 0;
    if (!look && !settings) return 0;
    at = photo_xmp_put(out, 0, size, photo_xmp_head);
    if (look) {
        at = photo_xmp_put(out, at, size, " xmlns:dc=\"http://purl.org/dc/elements/1.1/\" grkit:FilmLook=\"");
        at = photo_xmp_text(out, at, size, look, max);
        at = photo_xmp_put(out, at, size, "\"");
    }
    if (settings) {
        at = photo_xmp_properties(h, out, at, size);
        at = photo_xmp_state(h, out, at, size);
    }
    if (look) {
        at = photo_xmp_put(out, at, size, "><dc:subject><rdf:Bag><rdf:li>");
        at = photo_xmp_text(out, at, size, look, max);
        at = photo_xmp_put(out, at, size, "</rdf:li></rdf:Bag></dc:subject></rdf:Description>");
    } else {
        at = photo_xmp_put(out, at, size, "/>");
    }
    at = photo_xmp_put(out, at, size, photo_xmp_tail);
    return at <= size ? at : 0;
}

/* Whether `name` is at `xmp + i`. */
static __attribute__((unused)) u32 photo_xmp_at(const char *xmp, u32 len, u32 i, const char *name, u32 m) {
    u32 k;
    if (i > len || m > len - i) return 0;
    for (k = 0; k < m; ++k)
        if (xmp[i + k] != name[k]) return 0;
    return 1;
}
/* The value of property `name` in `xmp` of `len` bytes, as an attribute
   (name="..." after a blank) or as an element (<name>...</name>): its first
   byte, or 0, with its length in *n. */
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
/* grkit:State of `xmp` into the copy of a development that starts: 1, or 0
   without it. A value that the setting does not offer is left out. */
static __attribute__((unused)) u32 photo_xmp_apply(volatile u32 *h, const char *xmp, u32 len) {
    volatile u32 *t = settings_table(h), *c, *word;
    const char *s;
    u32 n, at, id, value, i, k;
    if (!h[GRKIT_HOST_SERVICES / 4] || (s = photo_xmp_find(xmp, len, "grkit:State", &n)) == 0) return 0;
    c = services_word(h, SERVICES_DEVELOP);
    for (at = 0; photo_xmp_next(s, n, &at, &id, &value);) {
        if (!id) continue;
        for (i = 0; i < GRKIT_CORE_SETTING_COUNT; ++i)
            if (photo_xmp_core_ids[i] == id) break;
        if (i < GRKIT_CORE_SETTING_COUNT) {
            if (value >= core_setting_values[i]) continue;
            word = c + DEVELOP_COPY_HEADER / 4;
            for (k = 0; k < DEVELOP_COPY_HEADER_WORDS; ++k)
                if (develop_copy_words[k] == core_setting_words[i]) word[k] = value;
            continue;
        }
        for (i = 0; t && i < t[0] && i < GRKIT_MODULE_SETTINGS_MAX; ++i)
            if (t[SETTINGS_ENTRY(i)] == id) {
                if (value < t[SETTINGS_ENTRY(i) + 2]) c[DEVELOP_COPY_MODULE / 4 + i] = value;
                break;
            }
    }
    return 1;
}

/* The XMP packet (tag 700) of IFD0 of the TIFF file in `buf`, of which
   `len` bytes are read: its offset, with its length in *n, or 0. */
static __attribute__((unused)) u32 photo_xmp_tiff(const u8 *buf, u32 len, u32 *n) {
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
