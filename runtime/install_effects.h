#ifndef GRKIT_INSTALL_EFFECTS_H
#define GRKIT_INSTALL_EFFECTS_H
/* A part of the installer: the listed effects, their choices, and their
   records. installer.c includes it once, in its order, after the helpers and
   the state that this part uses. */
/* The number of effect files past EFFECTS_LISTED, for REPORT.TXT. */
static u32 overflow_count;
/* Offer the effect in `path`, under its file name `name`. */
static void offer_path(const char *path, const char *name)
{
    u32 size=read_effect(path,temporary);
    if(!size || !valid_profile(temporary,size)) return;
    listed_count=catalog_offer(&listed[0][0],LISTED_HEAD,names,listed_count,EFFECTS_LISTED + 1,temporary,name);
    /* The catalog keeps the smallest IDs: the one past EFFECTS_LISTED is out. */
    if(listed_count>EFFECTS_LISTED) { ++overflow_count; listed_count=EFFECTS_LISTED; }
}
/* A symbolic link or an unusable file does not stop the other effects. */
static void offer_effect(const char *name, u32 n, void *context)
{
    if(!catalog_name(name,n)) return;
    char path[EFFECT_PATH_SIZE];
    if(join(path,sizeof(path),(const char *)context,name)) offer_path(path,name);
}
/* The state file ID of the choice of listed effect i: CORE.EFFECT.<its ID>. */
static u32 effect_choice_id(u32 i)
{
    char name[16 + CATALOG_NAME_SIZE + 32];
    u32 n = 0;
    for (const char *p = "CORE.EFFECT."; *p; ++p) name[n++] = *p;
    for (const u8 *p = listed[i] + CATALOG_ID; *p && n < sizeof(name) - 1; ++p) name[n++] = (char)*p;
    name[n] = 0;
    return grm_state_id(name);
}
/* Read the choices of the listed effects and order them: an effect with a
   position first, then the effects of the package, then the others, each
   group in ID order. Then install the effects that are not Off, in that
   order, then the effects that are Off while there is room, at most
   MAX_COUNT: an installed effect turns on or off at once
   (runtime/effects.h). A pending effect installs from the inbox. */
static struct state saved_state;
static void choose_effects(void)
{
    u32 keys[EFFECTS_LISTED], value;
    for (u32 i = 0; i < listed_count; ++i) {
        listed_choice[i] = state_value(&saved_state, effect_choice_id(i), &value) && value < EFFECT_CHOICE_VALUES ? value : 0;
        keys[i] = listed_choice[i] >> 1 ? listed_choice[i] >> 1
                  : (package_effect(names[i]) ? EFFECTS_DEFAULT_KEY : 2 * EFFECTS_DEFAULT_KEY) + i;
    }
    for (u32 i = 1; i < listed_count; ++i)
        for (u32 j = i; j && keys[j - 1] > keys[j]; --j) {
            u8 head[LISTED_HEAD];
            char file[CATALOG_NAME_SIZE];
            u32 t;
            copy(head, listed[j], LISTED_HEAD); copy(listed[j], listed[j - 1], LISTED_HEAD); copy(listed[j - 1], head, LISTED_HEAD);
            catalog_copy_name(file, names[j]); catalog_copy_name(names[j], names[j - 1]); catalog_copy_name(names[j - 1], file);
            t = keys[j]; keys[j] = keys[j - 1]; keys[j - 1] = t;
            t = listed_choice[j]; listed_choice[j] = listed_choice[j - 1]; listed_choice[j - 1] = t;
        }
    count = 0;
    for (u32 pass = 0; pass < 2; ++pass)
        for (u32 i = 0; i < listed_count; ++i) {
            char path[EFFECT_PATH_SIZE];
            u32 size, off = listed_choice[i] & 1;
            if (!pass) listed_record[i] = EFFECTS_NO_RECORD;
            if (off != pass) continue;
            listed_status[i] = off ? EFFECT_OFF : EFFECT_OVER;
            if (count == MAX_COUNT) continue;
            /* The file can change between the two reads: a profile with
               another ID does not install. */
            if (!join(path, sizeof(path), library_path, names[i]) || !(size = read_effect(path, profiles[count])))
                if (!join(path, sizeof(path), inbox_path, names[i]) || !(size = read_effect(path, profiles[count]))) size = 0;
            if (!size || !valid_profile(profiles[count], size) || !equal(profiles[count] + CATALOG_ID, listed[i] + CATALOG_ID, 32)) {
                listed_status[i] = EFFECT_UNREADABLE;
                continue;
            }
            if (!off) listed_status[i] = EFFECT_LOADED;
            listed_record[i] = count++;
        }
}
/* The effects of GRKIT/EFFECTS: GRKit lists at most EFFECTS_LISTED, those
   with the smallest IDs, and installs at most MAX_COUNT of them. */
static int tone_used(void);
static int discover(void)
{
    count=0; listed_count=0; overflow_count=0;
#ifdef GRKIT_NO_IMAGE_CONTROL
    /* Without Image Control, the target has no use for effect files. */
    return 0;
#endif
    /* Without a module that uses CORE.TONE, no effect installs
       (plan_modules). */
    if(!tone_used()) return 0;
    long fd=syscall6(5,(u32)library_path,0xc000,0,0,0,0);
    if(fd<0 && fd!=-2) return STATUS_LIBRARY_UNREADABLE;
    if(fd>=0) {
        static const struct directory_calls calls={directory_read,directory_seek};
        int status=directory_list(fd,directory_entries,sizeof(directory_entries),&calls,offer_effect,(void *)library_path);
        syscall6(6,fd,0,0,0,0,0);
        if(status) return status;
    }
    for(u32 i=0;i<pending_count;++i) {
        char path[EFFECT_PATH_SIZE];
        if(join(path,sizeof(path),inbox_path,pending[i])) offer_path(path,pending[i]);
    }
    choose_effects();
    return 0;
}
/* Photos record this code as their Image Control tag. It stays stable across
   catalogs and outside the native values 256..267. */
#ifdef GRKIT_NATIVE_TONE_PAGE
/* The target also records native values in this 256-value page. */
#define NATIVE_CODE(code) ((code) >> 8 == GRKIT_NATIVE_TONE_PAGE)
#else
#define NATIVE_CODE(code) 0
#endif
#ifndef GRKIT_NO_IMAGE_CONTROL
/* The MultiAxial parameter set of an effect with hue sectors (GRF feature 8,
   grkit_core::hue), at RECORD_MULTIAXIAL. The firmware reads a set as six
   pointers: bounds (16 u16, four descending ones per Cb/Cr quadrant), bank
   selectors (20 u8, five segments per quadrant), a matrix header whose word
   +0x10 points to 60 3x3 i32 Q10 matrices (12 banks x 5 levels), region values,
   21 u16 control fields, and the RGB to YCbCr matrix. The bounds and selectors
   give 11 equal sectors of the hue coordinate, sector k in bank k + 1; bank 0
   stays the identity. Without hue levels, every level of a bank gets the same
   matrix and control fields 1..8 = 0xfff keep every pixel on level 0. With them
   (feature 16), levels 1 to 3 come from the GRF, level 4 repeats level 3, and
   fields 1..8 are hue::LEVEL_CONTROL. Fields 10..20, the weight curve, come from
   the GRF. The region values and the YCbCr matrix are those of Standard's set
   (type 11). A native look (feature 32) gives its own bounds, selectors, all 21
   control fields, and the matrices of every bank and level (grf_native_matrix);
   a capture with highlight correction keeps that set. */
static const u16 multiaxial_bounds[16] = GRKIT_HUE_BOUNDS;
static const u8 multiaxial_banks[20] = GRKIT_HUE_SELECTORS;
static const u16 multiaxial_levels[8] = GRKIT_HUE_LEVEL_CONTROL;
static const u16 multiaxial_levels_halved[8] = GRKIT_HUE_LEVEL_CONTROL_HALVED;
static void multiaxial(u32 record, const u8 *profile)
{
    const u8 *sectors = profile + GRKIT_GRF_HUE;
    int levels = (get32(profile + GRKIT_GRF_FEATURES) & GRKIT_FEATURE_HUE_LEVELS) != 0;
    int native = (get32(profile + GRKIT_GRF_FEATURES) & GRKIT_FEATURE_NATIVE_LOOK) != 0;
    u32 slot = record + GRKIT_RECORD_MULTIAXIAL, table = slot + 0x98;
    u32 stock = word(TABLE_ADDRESS(1) + 11 * 4);
    store(slot, slot + 8);
    store(slot + 8, slot + 0x20);
    store(slot + 12, slot + 0x40);
    store(slot + 16, slot + 0x58);
    store(slot + 20, word(stock + 12));
    store(slot + 24, slot + 0x6c);
    store(slot + 28, word(stock + 20));
    copy(at(slot + 0x20), native ? profile + GRKIT_NATIVE_BOUNDS : (const u8 *)multiaxial_bounds, sizeof multiaxial_bounds);
    copy(at(slot + 0x40), native ? profile + GRKIT_NATIVE_SELECTORS : multiaxial_banks, sizeof multiaxial_banks);
    for (u32 k = 0; k < 5; ++k) store(slot + 0x58 + k * 4, table);
    volatile u16 *control = (volatile u16 *)at(slot + 0x6c);
    if (native) {
        const u8 *fields = profile + GRKIT_NATIVE_CONTROL;
        for (u32 k = 0; k < 21; ++k) control[k] = (u16)(fields[k * 2] | fields[k * 2 + 1] << 8);
        for (u32 bank = 0; bank < 12; ++bank)
            for (u32 level = 0; level < 5; ++level) {
                int m[9];
                grf_native_matrix(profile, bank, level, m);
                for (u32 k = 0; k < 9; ++k) store(table + (bank * 5 + level) * 36 + k * 4, (u32)m[k]);
            }
        store(slot + 4, slot + 8);
        return;
    }
    control[0] = 0;
    for (u32 k = 1; k <= 8; ++k) control[k] = levels ? multiaxial_levels[k - 1] : 0xfff;
    control[9] = 0;
    const u8 *fields = sectors + GRKIT_HUE_MATRICES;
    for (u32 k = 0; k < 11; ++k) control[10 + k] = (u16)(fields[k * 2] | fields[k * 2 + 1] << 8);
    for (u32 bank = 0; bank < 12; ++bank)
        for (u32 level = 0; level < 5; ++level) {
            /* Levels 1 to 3 follow GRKIT_HUE_LEVEL_OFFSET; level 4 repeats level 3. */
            u32 from = levels ? (level < 4 ? level : 3) : 0;
            const u8 *c = from ? profile + GRKIT_HUE_LEVEL_OFFSET + (from - 1) * GRKIT_HUE_MATRICES : sectors;
            int m[9] = {1024, 0, 0, 0, 1024, 0, 0, 0, 1024};
            for (u32 r = 0; bank && r < 3; ++r) {
                /* GRF validation keeps each diagonal within the 12-bit register. */
                int a = (signed char)c[(bank - 1) * 6 + r * 2] * 8;
                int b = (signed char)c[(bank - 1) * 6 + r * 2 + 1] * 8;
                m[r * 3 + (r + 1) % 3] = a;
                m[r * 3 + (r + 2) % 3] = b;
                m[r * 4] = 1024 - a - b;
            }
            for (u32 k = 0; k < 9; ++k) store(table + (bank * 5 + level) * 36 + k * 4, (u32)m[k]);
        }
    /* The set for a capture with highlight correction, which the hooks pick by the
       native flag. The camera then halves the stage's input, so these control fields
       select the levels and the weight of twice the luminance: the weight breakpoints
       halve and its slopes double (9-bit, clamped). The address at +4, the set at
       +0x908, its control fields at +0x920; the other pointers are those of the set. */
    u32 halved = slot + 0x908;
    store(slot + 4, halved);
    for (u32 k = 0; k < 6; ++k) store(halved + k * 4, word(slot + 8 + k * 4));
    store(halved + 16, slot + 0x920);
    volatile u16 *half = (volatile u16 *)at(slot + 0x920);
    half[0] = 0;
    for (u32 k = 1; k <= 8; ++k) half[k] = levels ? multiaxial_levels_halved[k - 1] : 0xfff;
    half[9] = 0;
    for (u32 k = 0; k < 4; ++k) {
        int slope = (int)(control[14 + k] & 0x1ff) - ((control[14 + k] & 0x100) ? 0x200 : 0);
        slope *= 2;
        if (slope > 255) slope = 255;
        if (slope < -256) slope = -256;
        half[10 + k] = control[10 + k];
        half[14 + k] = (u16)(slope & 0x1ff);
    }
    for (u32 k = 18; k <= 20; ++k) half[k] = (u16)(control[k] / 2);
}
#endif
static u32 effect_code(const u8 *id)
{
    return 0x8000 | (fnv(id, length((const char *)id)) & 0x7fff);
}
#endif
