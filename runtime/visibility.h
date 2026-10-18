#ifndef GRKIT_VISIBILITY_H
#define GRKIT_VISIBILITY_H
/* MODULES.TXT names the modules that the menu hides, separated by ASCII
   whitespace. The result has bit n set for module n of runtime/modules.h.
   Image Control is not listed: it shows exactly when effects are installed.
   grkit_core::modules implements the same rules. Callers define u8 and u32. */
#define VISIBILITY_FILE_MAX 256U
#define VISIBILITY_MODULES 4U
static const char *const visibility_names[VISIBILITY_MODULES] = {"framing", "guides", "date-stamp", "grain"};
static int visibility_space(u8 c) { return c==' ' || c=='\t' || c=='\r' || c=='\n'; }
static int visibility_name(const u8 *text, u32 n, const char *name)
{
    u32 i=0;
    while (i<n && name[i] && text[i]==(u8)name[i]) ++i;
    return i==n && !name[i];
}
/* Returns 0 for an unknown or repeated name. */
static int visibility_parse(const u8 *text, u32 n, u32 *hidden)
{
    u32 i=0, result=0;
    while (i<n) {
        u32 start, k=0;
        while (i<n && visibility_space(text[i])) ++i;
        if (i==n) break;
        start=i;
        while (i<n && !visibility_space(text[i])) ++i;
        while (k<VISIBILITY_MODULES && !visibility_name(text+start,i-start,visibility_names[k])) ++k;
        if (k==VISIBILITY_MODULES || (result & 2U<<k)) return 0;
        result |= 2U<<k;
    }
    *hidden=result;
    return 1;
}
#endif
