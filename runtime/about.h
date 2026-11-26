#ifndef GRKIT_ABOUT_H
#define GRKIT_ABOUT_H
/* About: the last root row, after the module pages. It is part of the core,
   so it always shows. Its summary gives the GRKit version. It has no rows, so
   OK on it opens no page. */
#include "core.h"

/* The About tile, GRKIT_ICON_VERSION, reads VER, so the GRKit root shows one
   GRK tile. */

static const u16 *about_text(volatile u32 *h) { return text(h,TEXT_VERSION); }
static const u16 *about_label(volatile u32 *h,u32 row) { (void)row; return about_text(h); }
static u32 about_none(volatile u32 *h) { (void)h; return 0; }
static u32 about_icon(volatile u32 *h,u32 row) { (void)h; (void)row; return GRKIT_ICON_VERSION; }
static u32 about_checked(volatile u32 *h,u32 row) { (void)h; (void)row; return 0; }
static void about_apply(volatile u32 *h,u32 row) { (void)h; (void)row; }

#define ABOUT_PAGE {about_text,about_text,about_none,about_label,about_icon, \
    about_checked,about_apply,about_none,0,0,0}
#endif
