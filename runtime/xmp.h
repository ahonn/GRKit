#ifndef GRKIT_XMP_H
#define GRKIT_XMP_H
/* The XMP packet that names the look of a JPEG: an APP1 segment after the
   APP1 Exif segment, with the look name as one keyword (dc:subject).
   Lightroom and other catalogs read keywords; they do not read the maker
   note.

   The name comes from the ImageTone code (maker-note tag 0x4f) in the Exif
   block of the same header: the name of the GRKit effect with that code, or
   the name of a native look. The tone hook of the shot wrote the code
   (runtime/picture.S: picture_capture_tone), and a rewrite of the file in the
   camera copies it from the file. Thus the name always matches the code of
   its own file, and two shots cannot exchange names.

   The includer defines u8, u16, and u32. Nothing here reads firmware state. */

/* Room that stays free after the segment: the APP7 preview segment that the
   firmware adds next is at most 0xffff + 2 bytes. */
#define XMP_APP7_RESERVE 0x10002U
/* The APP1 Exif setup of the firmware starts APP7 on 32 bytes. The
   segment keeps that, with blanks before the packet end. */
#define XMP_ALIGN 32U
/* The maker note of the GR III and GR IV families: "RICOH\0II", then an IFD
   with offsets from the TIFF header. */
#define XMP_MAKERNOTE_TAG 0x927cU
#define XMP_EXIF_IFD_TAG 0x8769U
#define XMP_TONE_TAG 0x4fU

static const char xmp_namespace[] = "http://ns.adobe.com/xap/1.0/";
static const char xmp_head[] =
    "<?xpacket begin=\"\xef\xbb\xbf\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>"
    "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">"
    "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">"
    "<rdf:Description rdf:about=\"\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\">"
    "<dc:subject><rdf:Bag><rdf:li>";
static const char xmp_tail[] =
    "</rdf:li></rdf:Bag></dc:subject>"
    "</rdf:Description></rdf:RDF></x:xmpmeta>";
static const char xmp_end[] = "<?xpacket end=\"w\"?>";

static __attribute__((unused)) u32 xmp_u16(const volatile u8 *p, u32 little) {
    return little ? (u32)p[0] | (u32)p[1] << 8 : (u32)p[0] << 8 | (u32)p[1];
}
static __attribute__((unused)) u32 xmp_u32(const volatile u8 *p, u32 little) {
    return little ? xmp_u16(p, 1) | xmp_u16(p + 2, 1) << 16 : xmp_u16(p, 0) << 16 | xmp_u16(p + 2, 0);
}
/* The value of `tag` in the IFD at `ifd` of a TIFF block of `size` bytes:
   1 with the value field (or its first SHORT) in `out`, or 0. */
static __attribute__((unused)) u32 xmp_ifd_value(const volatile u8 *tiff, u32 size, u32 ifd, u32 little,
                                                 u32 tag, u32 *out) {
    u32 count, i, entry, type;
    if (size < 2 || ifd > size - 2) return 0;
    count = xmp_u16(tiff + ifd, little);
    if (count > (size - ifd - 2) / 12) return 0;
    for (i = 0; i < count; ++i) {
        entry = ifd + 2 + i * 12;
        if (xmp_u16(tiff + entry, little) != tag) continue;
        type = xmp_u16(tiff + entry + 2, little);
        *out = type == 3 ? xmp_u16(tiff + entry + 8, little) : xmp_u32(tiff + entry + 8, little);
        return 1;
    }
    return 0;
}
/* The ImageTone code in the maker note of the TIFF block at `tiff`, which
   ends before `size`: 1 with the code in `code`, or 0 without a Ricoh maker
   note or the tag. */
static __attribute__((unused)) u32 xmp_tone_code(const volatile u8 *tiff, u32 size, u32 *code) {
    static const char ricoh[8] = {'R', 'I', 'C', 'O', 'H', 0, 'I', 'I'};
    u32 little, exif, note, i;
    if (size < 8) return 0;
    if (tiff[0] == 'I' && tiff[1] == 'I') little = 1;
    else if (tiff[0] == 'M' && tiff[1] == 'M') little = 0;
    else return 0;
    if (!xmp_ifd_value(tiff, size, xmp_u32(tiff + 4, little), little, XMP_EXIF_IFD_TAG, &exif)) return 0;
    if (!xmp_ifd_value(tiff, size, exif, little, XMP_MAKERNOTE_TAG, &note)) return 0;
    if (note > size - 10) return 0;
    for (i = 0; i < 8; ++i)
        if (tiff[note + i] != (u8)ricoh[i]) return 0;
    return xmp_ifd_value(tiff, size, note + 8, 1, XMP_TONE_TAG, code);
}
/* The next code point of a UTF-16 name of at most `max` units, from unit
   `*at`, or 0 at its end. A lone surrogate becomes U+FFFD. */
static __attribute__((unused)) u32 xmp_code_point(const volatile u16 *name, u32 max, u32 *at) {
    u32 c, d;
    if (*at >= max || !(c = name[*at])) return 0;
    ++*at;
    if (c < 0xd800 || c > 0xdfff) return c;
    if (c < 0xdc00 && *at < max && (d = name[*at]) >= 0xdc00 && d <= 0xdfff) {
        ++*at;
        return 0x10000 + ((c - 0xd800) << 10) + (d - 0xdc00);
    }
    return 0xfffd;
}
/* The name as escaped UTF-8 into `out` (0 to count only): its length. A
   control character becomes a blank. */
static __attribute__((unused)) u32 xmp_put_name(volatile u8 *out, const volatile u16 *name, u32 max) {
    u32 at = 0, n = 0, c, k;
    const char *escape;
    while ((c = xmp_code_point(name, max, &at))) {
        escape = c == '&' ? "&amp;" : c == '<' ? "&lt;" : c == '>' ? "&gt;" : c == '"' ? "&quot;" : 0;
        if (escape) {
            for (k = 0; escape[k]; ++k, ++n)
                if (out) out[n] = (u8)escape[k];
            continue;
        }
        if (c < 0x20) c = ' ';
        if (c < 0x80) {
            if (out) out[n] = (u8)c;
            n += 1;
        } else if (c < 0x800) {
            if (out) { out[n] = (u8)(0xc0 | c >> 6); out[n + 1] = (u8)(0x80 | (c & 0x3f)); }
            n += 2;
        } else if (c < 0x10000) {
            if (out) { out[n] = (u8)(0xe0 | c >> 12); out[n + 1] = (u8)(0x80 | (c >> 6 & 0x3f));
                       out[n + 2] = (u8)(0x80 | (c & 0x3f)); }
            n += 3;
        } else {
            if (out) { out[n] = (u8)(0xf0 | c >> 18); out[n + 1] = (u8)(0x80 | (c >> 12 & 0x3f));
                       out[n + 2] = (u8)(0x80 | (c >> 6 & 0x3f)); out[n + 3] = (u8)(0x80 | (c & 0x3f)); }
            n += 4;
        }
    }
    return n;
}
static __attribute__((unused)) u32 xmp_put(volatile u8 *out, u32 n, const char *text, u32 length) {
    u32 i;
    for (i = 0; i < length; ++i) out[n + i] = (u8)text[i];
    return n + length;
}
/* The bytes of the APP1 segment of a name, marker included, on XMP_ALIGN;
   0 for an empty name or a segment over the 64 KiB limit. */
static __attribute__((unused)) u32 xmp_segment_size(const volatile u16 *name, u32 max) {
    u32 text = xmp_put_name(0, name, max), body;
    if (!text) return 0;
    body = sizeof(xmp_namespace) + sizeof(xmp_head) - 1 + text + sizeof(xmp_tail) - 1 + sizeof(xmp_end) - 1;
    body = (4 + body + XMP_ALIGN - 1) & ~(XMP_ALIGN - 1);
    return body - 2 <= 0xffffU ? body : 0;
}
/* Writes the segment of `size` bytes (xmp_segment_size) at `out`. */
static __attribute__((unused)) void xmp_write_segment(volatile u8 *out, u32 size, const volatile u16 *name, u32 max) {
    u32 n = 4, text;
    out[0] = 0xff; out[1] = 0xe1; out[2] = (u8)((size - 2) >> 8); out[3] = (u8)(size - 2);
    n = xmp_put(out, n, xmp_namespace, sizeof(xmp_namespace));
    n = xmp_put(out, n, xmp_head, sizeof(xmp_head) - 1);
    text = xmp_put_name(out + n, name, max); n += text;
    n = xmp_put(out, n, xmp_tail, sizeof(xmp_tail) - 1);
    while (n < size - (sizeof(xmp_end) - 1)) out[n++] = ' ';
    xmp_put(out, n, xmp_end, sizeof(xmp_end) - 1);
}
/* The names of the native looks, in the order of their ImageTone codes
   from XMP_NATIVE_FIRST, as the camera menu shows them. GR IV adds the two
   Cinema looks (types 18 and 19) after the looks of GR III. They take the
   names of the GRKit effects that GR III gets from them, so one keyword finds
   both. */
#define XMP_NATIVE_FIRST 0x100U
static const char xmp_native_names[] =
    "Standard\0Vivid\0Monotone\0Soft Monotone\0Hard Monotone\0Hi-Contrast B&W\0"
    "Positive Film\0Bleach Bypass\0Retro\0HDR Tone\0Cross Processing\0Negative Film\0"
    "Cinema Yellow\0Cinema Green\0";
#ifdef NATIVE_TONE_MONOCHROME
/* The looks of GR IV Monochrome from NATIVE_TONE_MONOCHROME. Monochrome
   before each name keeps them apart from the color looks of the same name. */
static const char xmp_monochrome_names[] =
    "Monochrome Standard\0Monochrome Solid\0Monochrome Soft\0Monochrome Hi-Contrast\0"
    "Monochrome Grainy\0Monochrome HDR Tone\0Monochrome Solarized Effect\0";
#endif
/* The longest name with its NUL, in UTF-16 units. */
#define XMP_NATIVE_UNITS 32U
/* Name `index` of a list of `size` bytes as UTF-16 in `name` (`max` units
   with its NUL): 1, or 0 past the end of the list. */
static __attribute__((unused)) u32 xmp_list_name(const char *list, u32 size, u32 index, u16 *name, u32 max) {
    const char *p = list, *end = list + size - 1;
    u32 i;
    for (; index; --index) {
        while (*p) ++p;
        if (++p >= end) return 0;
    }
    for (i = 0; p[i] && i + 1 < max; ++i) name[i] = (u8)p[i];
    if (p[i]) return 0;
    name[i] = 0;
    return 1;
}
/* The name of the native look with ImageTone `code` as UTF-16 in `name`
   (`max` units with its NUL): 1, or 0 for a code without a name. */
static __attribute__((unused)) u32 xmp_native_name(u32 code, u16 *name, u32 max) {
#ifdef NATIVE_TONE_MONOCHROME
    if (code >= NATIVE_TONE_MONOCHROME && code < NATIVE_TONE_MONOCHROME + 0x100U)
        return xmp_list_name(xmp_monochrome_names, sizeof(xmp_monochrome_names), code - NATIVE_TONE_MONOCHROME, name, max);
#endif
    if (code < XMP_NATIVE_FIRST || code >= XMP_NATIVE_FIRST + 0x100U) return 0;
    return xmp_list_name(xmp_native_names, sizeof(xmp_native_names), code - XMP_NATIVE_FIRST, name, max);
}
#endif
