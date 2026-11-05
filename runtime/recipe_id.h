#ifndef GRKIT_RECIPE_ID_H
#define GRKIT_RECIPE_ID_H
/* The IDs of recipes (GRKIT_HOST_RECIPE_IDS): generated numbers, 1 to
   GRKIT_RECIPE_ID_MAX. The site generates the ID of a recipe that joins its
   library, and the camera (runtime/recipes.h) and INSTALL
   (runtime/install_recipes.h) the ID of a recipe that they create. So a
   recipe that the site moves to another card keeps an ID that no recipe of
   that card is likely to have had. Each creator takes the first candidate
   that is none of the IDs of the card and none of its user modes
   (GRKIT_HOST_RECIPE_DIAL), so a reference to a deleted recipe never names
   another. The includer defines u32 and the GRKIT_RECIPE_* values
   (runtime/abi.h). */

/* The candidates that a creator tries. Past them, the recipe goes without an
   ID, and INSTALL generates one at the next start. */
#define RECIPE_ID_ATTEMPTS 256U

/* The value behind a new ID, which differs between calls: on the camera,
   the 64-bit count of the generic timer (CNTPCT, 40 MHz). INSTALL, a Linux
   process that cannot read the timer, defines RECIPE_ID_SOURCE from
   clock_gettime (runtime/installer.c). A host build has no camera timer,
   and a test defines RECIPE_ID_SOURCE. */
#ifndef RECIPE_ID_SOURCE
#ifdef __arm__
static u32 recipe_id_source(u32 *high) {
    u32 low, hi;
    __asm__ volatile("mrrc p15, 0, %0, %1, c14" : "=r"(low), "=r"(hi));
    *high = hi;
    return low;
}
#else
static u32 recipe_id_source(u32 *high) { *high = 0; return 0; }
#endif
#define RECIPE_ID_SOURCE(high) recipe_id_source(high)
#endif

/* Candidate `attempt` for a new ID: the source and the attempt, mixed by the
   finalizer of MurmurHash3, in 29 bits. A candidate that is 0 or above
   GRKIT_RECIPE_ID_MAX is no ID. */
static u32 recipe_id_candidate(u32 attempt) {
    u32 high, x = RECIPE_ID_SOURCE(&high);
    x ^= high * 0x9e3779b9U ^ (attempt + 1U) * 0x85ebca6bU;
    x ^= x >> 16;
    x *= 0x85ebca6bU;
    x ^= x >> 13;
    x *= 0xc2b2ae35U;
    x ^= x >> 16;
    return x & 0x1fffffffU;
}
_Static_assert(GRKIT_RECIPE_ID_MAX + 1U == 0x1fffffffU, "a candidate has the bits of an ID and of GRKIT_RECIPE_DIAL_OFF");
_Static_assert(GRKIT_RECIPE_DIAL_OFF > GRKIT_RECIPE_ID_MAX, "no ID turns GRKit off");
#endif
