#include <assert.h>
#include <string.h>
static unsigned framing_requests;
#define MENU_FRAMING_REQUEST() (++framing_requests)
#include "menu.c"
#include "menu_labels.h"

static volatile u32 test_header[64];
static u32 native_changes;
static u32 native_events;
volatile u32 *menu_header(void) { return test_header; }
u32 menu_event_original(u32 controller, const u16 *event)
{ (void)controller; (void)event; ++native_events; return 42; }
void menu_exit_original(u32 controller) { (void)controller; }
u32 menu_destroy_original(u32 controller) { return controller; }
void menu_refresh_original(u32 controller) { (void)controller; }
void menu_changed_original(u32 controller) { (void)controller; ++native_changes; }

int main(void)
{
    volatile u32 text[2];
    descriptor(text, plugins);
    assert(text[0] == 0x106);
    descriptor(text, off_summary);
    assert(text[0] == 0x113);
    {
        u16 summary[GRKIT_SUMMARY_UNITS + 1];
        const char *names[] = {"Ektar 100", "1234567890ABCDEF"};
        for (u32 i = 0; i < 2; ++i) {
            memset(summary, 0xff, sizeof(summary));
            module_summary(summary, (const u8 *)names[i]);
            const char *prefix = "Image Control: ";
            u32 n = strlen(prefix), m = strlen(names[i]);
            for (u32 j = 0; j < n; ++j) assert(summary[j] == prefix[j]);
            for (u32 j = 0; j < m; ++j) assert(summary[n+j] == names[i][j]);
            assert(summary[n+m] == 0);
            assert(summary[GRKIT_SUMMARY_UNITS] == 0xffff);
            descriptor(text, summary);
            assert(text[0] == (0x100 | (n+m+1)));
        }
    }
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    test_header[GRKIT_HOST_OFF_ICON / 4] = 0x59001000;
    test_header[GRKIT_HOST_PLUGINS_ICON / 4] = 0x59003000;
    test_header[GRKIT_HOST_RECORDS / 4] = 0x59005000;
    assert(selected_icon(test_header) == 0x7fff);
    assert(menu_viewfinder_icon_select(0x166, 0) == 0x166);
    assert(menu_viewfinder_icon_select(0, 0) == 0);
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 3 * GRKIT_RECORD_SIZE;
    assert(selected_icon(test_header) == 0x8003);
    assert(menu_viewfinder_icon_select(0x166, 0) == 0x8003);
    assert(menu_viewfinder_icon_select(0x166, 1) == 0x166);
    test_header[GRKIT_HOST_ACTIVE / 4] += 4;
    assert(menu_viewfinder_icon_select(0x167, 0) == 0x167);
    test_header[GRKIT_HOST_ACTIVE / 4] = 0;
    /* Fn and its hint apply to effect rows of Image Control only. */
    test_header[9] = MENU_MODULE;
    test_header[10] = 0;
    assert(!parameters_row(test_header));
    test_header[10] = 2;
    assert(parameters_row(test_header));
    test_header[GRKIT_HOST_MODULE / 4] = 1;
    assert(!parameters_row(test_header));
    test_header[GRKIT_HOST_MODULE / 4] = 0;
    test_header[9] = MENU_ROOT;
    assert(!parameters_row(test_header));
    test_header[9] = 0;
    test_header[10] = 0;
    assert(menu_press(2, 0));
    assert(!menu_press(1, 0));
    assert(!menu_press(2, 1));
    assert(!menu_press(2, 100));
    assert(back_event(test_header, 5, 1) == 1);
    assert(back_event(test_header, 5, 1) == 2);
    assert(back_event(test_header, 0x19d, 1) == 0);
    assert(back_event(test_header, 0xd8, 0) == 2);
    assert(back_event(test_header, 0xd8, 0) == 0);
    assert(back_event(test_header, 5, 0) == 0);
    assert(back_event(test_header, 5, 1) == 1);
    assert(back_event(test_header, 8, 1) == 0);
    assert(back_event(test_header, 0xd8, 1) == 0);
    /* List restoration changes level to native before rebuilding the rows. */
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000;
    test_header[8] = 0x1000;
    test_header[15] = 1;
    menu_changed(0x1000);
    assert(native_changes == 0);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    test_header[9] = 1;
    menu_changed(0x1000);
    assert(native_changes == 0);
    test_header[9] = 0;
    test_header[15] = 0;
    menu_changed(0x1000);
    assert(native_changes == 1);
    {
        const u16 confirm[] = {0xd8};
        test_header[9] = 0;
        test_header[GRKIT_HOST_BACK_PENDING / 4] = 1;
        assert(menu_event(0x1000, confirm) == 0);
        assert(native_events == 0);
        assert(test_header[GRKIT_HOST_BACK_PENDING / 4] == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* The event ring distinguishes MENU's confirm from OK and ADJ press confirms. */
    test_header[19] = 18;
    test_header[28] = 5 | (2 << 16);
    test_header[29] = 0xd8;
    assert(after_menu_switch(test_header));
    test_header[28] = 8;
    assert(!after_menu_switch(test_header));
    test_header[28] = 0x1e;
    assert(!after_menu_switch(test_header));
    test_header[19] = 0;
    test_header[28] = test_header[29] = 0;
    /* Playback matches only extracted non-native tags with the private code bit. */
    {
        u32 count = test_header[GRKIT_HOST_COUNT / 4];
        assert(playback_icon(test_header, 0) == 0);
        assert(playback_icon(test_header, 5) == 0);
        assert(playback_icon(test_header, 0x10000 | 0x7eff) == 0);
        assert(playback_icon(test_header, 0x20000 | 0x7f00) == 0);
        test_header[GRKIT_HOST_COUNT / 4] = 0;
        assert(playback_icon(test_header, 0x10000 | 0x7f00) == 0);
        test_header[GRKIT_HOST_COUNT / 4] = count;
    }
    assert(!native_action_release(10, 0));
    assert(!native_action_release(0, 1));
    assert(native_action_release(10, 1));
    assert(menu_icon(0x7ffe) == 0x59003000);
    assert(toolbar_x(0) == 415 && toolbar_x(5) == 665);
    assert(menu_icon(0x7fff) == 0x59001000);
    assert(menu_icon(0x166) == 0);
    assert(menu_icon(0x8005) == 0);
    assert(menu_icon(0xffff) == 0);
    assert(wrapped_edge(0xcf, 3, 0, 0) == 2);
    assert(wrapped_edge(0xce, 1, 4, 0) == 1);
    assert(wrapped_edge(0xcf, 1, 2, 0) == 0);
    assert(wrapped_edge(0xce, 3, 2, 0) == 0);
    assert(wrapped_edge(0xcf, 4, 0, 1) == 0);
    assert(wrapped_edge(0x35, 4, 0, 0) == 0);
    assert(advance_from_private(0xcf, 1) == 1);
    assert(advance_from_private(0xce, 1) == 0);
    assert(advance_from_private(0xcf, 2) == 0);
    assert(advance_from_private(0xce, 2) == 1);
    test_header[9] = 1;
    assert(count(test_header) == 4);
    test_header[9] = 2;
    assert(count(test_header) == 6);
    test_header[GRKIT_HOST_COUNT / 4] = 0;
    assert(count(test_header) == 1);
    /* Registry routing leaves the existing effect record untouched. */
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000;
    test_header[GRKIT_HOST_COUNT / 4] = 5;
    test_header[GRKIT_HOST_FRAMING_ICON / 4] = 0x59007000;
    /* Framing tiles are contiguous after the module tile. */
    assert(menu_icon(FRAMING_ICON) == 0x59007000);
    assert(menu_icon(FRAMING_RATIO_ICON(3)) == 0x59007000 + 3 * GRKIT_ICON_STRIDE);
    /* Guides tiles follow the Framing tiles. */
    assert(menu_icon(GUIDES_ICON) == 0x59007000 + FRAMING_ICON_COUNT * GRKIT_ICON_STRIDE);
    assert(menu_icon(GUIDE_STYLE_ICON(4)) == 0x59007000 + 8 * GRKIT_ICON_STRIDE);
    /* Module tiles fill 0x7ff0..0x7ffd; the GRKit tile (0x7ffe) follows them. */
    assert(menu_icon(FRAMING_ICON - 1) == 0);
    assert(FRAMING_ICON + MODULE_ICON_COUNT <= 0x7ffe);
    /* Date Stamp tiles follow the Guides tiles. */
    assert(menu_icon(STAMP_ICON) == 0x59007000 + 9 * GRKIT_ICON_STRIDE);
    assert(menu_icon(STAMP_STYLE_ICON(4)) == 0x59007000 + 13 * GRKIT_ICON_STRIDE);
    test_header[9] = MENU_ROOT;
    test_header[10] = 1;
    assert(current_module(test_header) == &modules[1]);
    assert(current_module(test_header)->summary(test_header) == frame_summaries[0]);
    assert(modules[1].icon(test_header, 1) == FRAMING_ICON);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 2;
    assert(modules[1].icon(test_header, 1) == FRAMING_RATIO_ICON(2));
    /* Framing lists the ratios directly and opens at the selected one. */
    menu_confirm(test_header);
    assert(test_header[9] == MENU_MODULE && test_header[10] == 2);
    assert(count(test_header) == 4);
    assert(current_module(test_header)->title(test_header) == framing);
    for (u32 row = 0; row < 4; ++row) {
        test_header[10] = row;
        menu_confirm(test_header);
        assert(test_header[9] == MENU_MODULE);
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == row);
        assert(framing_requests == row + 1);
        assert(current_module(test_header)->checked(test_header, row));
        assert(!current_module(test_header)->checked(test_header, (row + 1) % 4));
        assert(current_module(test_header)->label(test_header, row) == ratio_labels[row]);
        assert(current_module(test_header)->icon(test_header, row) == (row ? FRAMING_RATIO_ICON(row) : 0x7fff));
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* The Framing page shows its highlighted row on the ADJ canvas. */
    test_header[10] = 1;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 2);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5a000000);
    test_header[10] = 0;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    frame_preview(test_header, 0x5a000000, 1);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    test_header[10] = 3;
    assert(menu_back(test_header));
    assert(test_header[9] == MENU_ROOT && test_header[10] == 1);
    /* Other ADJ pages show the committed ratio, here 4:3. */
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 4);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5b000000);
    /* A committed Off ends the mask but keeps the canvas for one erasing draw. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5b000000);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 3;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    assert(!menu_back(test_header));
    assert(current_module(test_header)->summary(test_header) == frame_summaries[3]);
    /* Guides opens at the committed style and commits only its own setting. */
    test_header[9] = MENU_ROOT;
    test_header[10] = 2;
    assert(current_module(test_header)->summary(test_header) == guide_summaries[0]);
    assert(modules[2].icon(test_header, 2) == GUIDES_ICON);
    test_header[GRKIT_HOST_GUIDE / 4] = 3;
    assert(modules[2].icon(test_header, 2) == GUIDE_STYLE_ICON(3));
    {
        u32 requests = framing_requests;
        menu_confirm(test_header);
        assert(test_header[9] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == GUIDES_MODULE);
        assert(test_header[10] == 3);
        assert(count(test_header) == 5);
        assert(current_module(test_header)->title(test_header) == guides);
        for (u32 row = 0; row < 5; ++row) {
            test_header[10] = row;
            menu_confirm(test_header);
            assert(test_header[GRKIT_HOST_GUIDE / 4] == row);
            assert(current_module(test_header)->checked(test_header, row));
            assert(!current_module(test_header)->checked(test_header, (row + 1) % 5));
            assert(current_module(test_header)->label(test_header, row) == guide_labels[row]);
            assert(current_module(test_header)->icon(test_header, row) == (row ? GUIDE_STYLE_ICON(row) : 0x7fff));
            assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 3);
            assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
        }
        assert(framing_requests == requests);
    }
    /* The Guides page previews its highlighted style inside the committed ratio. */
    test_header[10] = 2;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 4);
    assert(test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 2);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] == 0x5a000000);
    /* A highlighted Off with no ratio keeps the canvas for the next row. */
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 0;
    test_header[10] = 0;
    frame_preview(test_header, 0x5a000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    assert(test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 0);
    assert(menu_back(test_header));
    assert(test_header[9] == MENU_ROOT && test_header[10] == 2);
    /* Other ADJ pages show the committed style, even without a ratio. */
    test_header[GRKIT_HOST_GUIDE / 4] = 4;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 1);
    assert(test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 4);
    /* The Framing page keeps the committed style with its highlighted ratio. */
    test_header[10] = 1;
    menu_confirm(test_header);
    test_header[10] = 2;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 3);
    assert(test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 4);
    /* Committed Off for both ends the drawing. */
    assert(menu_back(test_header));
    test_header[GRKIT_HOST_GUIDE / 4] = 0;
    frame_preview(test_header, 0x5b000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 0);
    test_header[GRKIT_HOST_GUIDE / 4] = 999;
    assert(guide_style(test_header) == 0);
    test_header[GRKIT_HOST_GUIDE / 4] = 0;
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 3;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    /* Date Stamp lists its styles, opens at the committed one, and changes nothing else. */
    test_header[9] = MENU_ROOT;
    test_header[10] = 3;
    assert(current_module(test_header)->summary(test_header) == stamp_summaries[0]);
    assert(modules[3].icon(test_header, 3) == STAMP_ICON);
    test_header[GRKIT_HOST_STAMP / 4] = 2;
    assert(modules[3].icon(test_header, 3) == STAMP_STYLE_ICON(2));
    menu_confirm(test_header);
    assert(test_header[9] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == STAMP_MODULE);
    assert(test_header[10] == 2 && count(test_header) == 5);
    assert(current_module(test_header)->title(test_header) == date_stamp);
    for (u32 row = 0; row < 5; ++row) {
        test_header[10] = row;
        menu_confirm(test_header);
        assert(test_header[GRKIT_HOST_STAMP / 4] == row);
        assert(current_module(test_header)->checked(test_header, row));
        assert(current_module(test_header)->label(test_header, row) == stamp_labels[row]);
        assert(current_module(test_header)->icon(test_header, row) == (row ? STAMP_STYLE_ICON(row) : 0x7fff));
        assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 3 && test_header[GRKIT_HOST_GUIDE / 4] == 0);
        assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    }
    /* The Date Stamp page keeps the committed mask and guides on the ADJ canvas. */
    frame_preview(test_header, 0x5c000000, 0);
    assert(test_header[GRKIT_HOST_FRAME_PREVIEW / 4] == 4 && test_header[GRKIT_HOST_GUIDE_PREVIEW / 4] == 0);
    assert(menu_back(test_header) && test_header[10] == 3);
    assert(current_module(test_header)->summary(test_header) == stamp_summaries[4]);
    test_header[GRKIT_HOST_STAMP / 4] = 999;
    assert(stamp_style(test_header) == 0);
    test_header[GRKIT_HOST_STAMP / 4] = 0;
    test_header[GRKIT_HOST_FRAME_PREVIEW_CANVAS / 4] = 0;
    test_header[9] = MENU_ROOT;
    test_header[10] = 1;
    /* Invalid public navigation cursors cannot form an effect pointer. */
    test_header[10] = 999;
    menu_confirm(test_header);
    assert(test_header[9] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == 0);
    test_header[10] = 999;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0);
    test_header[10] = 5;
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 4 * GRKIT_RECORD_SIZE);
    assert(test_header[GRKIT_HOST_FRAME_RATIO / 4] == 3);
    assert(menu_back(test_header));
    assert(test_header[10] == 0);
    test_header[GRKIT_HOST_MODULE / 4] = 999;
    test_header[9] = MENU_MODULE + 1;
    menu_normalize(test_header);
    assert(test_header[9] == MENU_ROOT && test_header[GRKIT_HOST_MODULE / 4] == 0);
    test_header[GRKIT_HOST_FRAME_RATIO / 4] = 999;
    assert(frame_ratio(test_header) == 0);
    /* Pages start at multiples of PAGE for every supported catalog row. */
    for (u32 row = 0; row <= 146; ++row) assert(page_start(row) == row / PAGE * PAGE);

    /* Image Control opens at the committed row. */
    test_header[GRKIT_HOST_PREVIEW / 4] = 0;
    test_header[GRKIT_HOST_ACTIVE / 4] = 0x59005000 + 3 * GRKIT_RECORD_SIZE;
    test_header[9] = MENU_ROOT;
    test_header[10] = 0;
    menu_confirm(test_header);
    assert(test_header[9] == MENU_MODULE && test_header[GRKIT_HOST_MODULE / 4] == 0);
    assert(test_header[10] == 4);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    /* Highlighting previews in the live view without committing. Off previews native. */
    test_header[10] = 2;
    image_preview(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0x59005000 + GRKIT_RECORD_SIZE);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 3 * GRKIT_RECORD_SIZE);
    test_header[10] = 0;
    image_preview(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 1);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 3 * GRKIT_RECORD_SIZE);
    /* MENU restores the committed effect. */
    assert(menu_back(test_header));
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000 + 3 * GRKIT_RECORD_SIZE);
    /* OK commits the highlighted row and ends the preview. */
    menu_confirm(test_header);
    test_header[10] = 1;
    image_preview(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0x59005000);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0x59005000);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    test_header[10] = 0;
    image_preview(test_header);
    menu_confirm(test_header);
    assert(test_header[GRKIT_HOST_ACTIVE / 4] == 0);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    /* Off opens Image Control at the Off row. */
    assert(menu_back(test_header));
    menu_confirm(test_header);
    assert(test_header[10] == 0);
    /* Other levels and modules do not preview effects. */
    assert(menu_back(test_header));
    image_preview(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    test_header[10] = 1;
    menu_confirm(test_header);
    image_preview(test_header);
    assert(test_header[GRKIT_HOST_PREVIEW / 4] == 0);
    return 0;
}
