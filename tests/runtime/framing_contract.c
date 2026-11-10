#include <assert.h>
#include <stdint.h>
#include <string.h>

#define TEST_OWNER 0x1000U
#define TEST_CANVAS 0x2000U
#define TEST_BITMAP 0x3000U
#define TEST_PIXELS 0x60000000U
#define TEST_STRIDE 736U
#define SENTINEL 0x76543210U
/* The live view controller reaches LIVEVIEW_LEVEL_PENDING (below 0x800). */
static uint32_t owner_memory[512], canvas_memory[256], bitmap_memory[256];
/* One effect record for the highlight warning (GRKIT_RECORD_SIZE bytes). */
#define TEST_RECORD 0x5d000000U
static uint32_t record_memory[0x1100 / 4];
/* The playback metadata: the size of the photo in playback at METADATA_SIZE. */
#define TEST_METADATA 0x5c000000U
static uint32_t metadata_memory[0x40 / 4];
static uint32_t pixels[1024 * 480 + 2];
static uint32_t native_aspect, capture_aspect, capture_requests;
#define TEST_INDICATOR 0x5b000000U
#define TEST_ICONS 0x5a000000U
#define TEST_LAYERS 0x59000000U
static uint32_t layer_memory[128/4];
static uint32_t indicator_memory[256/4], icon_memory[40*6412/4];
/* Native views of the canvas: TEST_VIEW_COUNT views of TEST_VIEW_SIZE bytes,
   then their child vectors. */
#define TEST_VIEWS 0x57000000U
#define TEST_VIEW_SIZE 0xa0U
#define TEST_VIEW_COUNT 9U
#define TEST_VECTORS (TEST_VIEWS+TEST_VIEW_COUNT*TEST_VIEW_SIZE)
static uint32_t view_memory[(TEST_VIEW_COUNT*TEST_VIEW_SIZE+0x100)/4];

static void *mapped(uint32_t address, uint32_t bytes)
{
    uint32_t *object, size = sizeof(canvas_memory);
    if (address >= TEST_VIEWS && address < TEST_VIEWS + sizeof(view_memory)) {
        object = view_memory; address -= TEST_VIEWS; size = sizeof(view_memory);
    } else if (address >= TEST_LAYERS && address < TEST_LAYERS + sizeof(layer_memory)) {
        object = layer_memory; address -= TEST_LAYERS; size = sizeof(layer_memory);
    } else if (address >= TEST_INDICATOR && address < TEST_INDICATOR + sizeof(indicator_memory)) {
        object = indicator_memory; address -= TEST_INDICATOR; size = sizeof(indicator_memory);
    } else if (address >= TEST_ICONS && address < TEST_ICONS + sizeof(icon_memory)) {
        object = icon_memory; address -= TEST_ICONS; size = sizeof(icon_memory);
    } else if (address >= TEST_OWNER && address < TEST_OWNER + sizeof(owner_memory)) {
        object = owner_memory; address -= TEST_OWNER; size = sizeof(owner_memory);
    } else if (address >= TEST_CANVAS && address < TEST_CANVAS + sizeof(canvas_memory)) {
        object = canvas_memory; address -= TEST_CANVAS;
    } else if (address >= TEST_RECORD && address < TEST_RECORD + sizeof(record_memory)) {
        object = record_memory; address -= TEST_RECORD; size = sizeof(record_memory);
    } else if (address >= TEST_METADATA && address < TEST_METADATA + sizeof(metadata_memory)) {
        object = metadata_memory; address -= TEST_METADATA; size = sizeof(metadata_memory);
    } else {
        assert(address >= TEST_BITMAP && address < TEST_BITMAP + sizeof(bitmap_memory));
        object = bitmap_memory; address -= TEST_BITMAP;
    }
    assert(address + bytes <= size);
    assert(address % bytes == 0);
    return (uint8_t *)object + address;
}
static volatile uint32_t *mapped_pixels(uint32_t address)
{
    assert(address >= TEST_PIXELS && (address & 3) == 0);
    uint32_t offset = (address - TEST_PIXELS) / 4;
    assert(offset + 720 <= 1024 * 480);
    return pixels + 1 + offset;
}
#define WORD(p,o) (*(uint32_t *)mapped((p)+(o),4))
#define HALF(p,o) (*(uint16_t *)mapped((p)+(o),2))
#define BYTE(p,o) (*(uint8_t *)mapped((p)+(o),1))
#define FRAMING_PIXELS(address) mapped_pixels(address)
#define FRAMING_NATIVE_ASPECT() native_aspect
/* The family header reads and requests the capture aspect in the firmware;
   the harness keeps it in capture_aspect. */
#include "firmware.h"
#undef FRAMING_CAPTURE_ASPECT
#undef FRAMING_REQUEST_ASPECT
#define FRAMING_CAPTURE_ASPECT() capture_aspect
/* The native setter resends 0 through the property handler hook. */
#define FRAMING_REQUEST_ASPECT() (++capture_requests, capture_aspect=framing_capture_request(0))
static uint32_t display_mode;
#define FRAMING_DISPLAY_MODE() display_mode
#define FRAMING_PLAYBACK_METADATA() TEST_METADATA
static uint32_t invalidations, invalidated;
#define FRAMING_INVALIDATE(view) (++invalidations, invalidated = (view))
static uint32_t local_second;
#define FRAMING_SECOND() local_second
/* The label table of the build, written by the test runner. The header puts
   it at TEST_LABELS. */
#include "abi.h"
#include "overlay_labels.h"
#define TEST_RECORDS 0x70000000U
#define TEST_LABELS (TEST_RECORDS+GRKIT_PICTURE_SCRATCH+GRKIT_TEXT_SIZE)
static const volatile uint8_t *mapped_labels(uint32_t address)
{
    assert(address == TEST_LABELS);
    return overlay_labels;
}
#define LABEL_MAP(address) mapped_labels(address)
/* The module settings table of the harness: the five Guides settings. */
#define TEST_SETTINGS 0x5f000000U
static uint32_t settings_memory[2 + 3 * 5];
static volatile uint32_t *mapped_settings(uint32_t address)
{
    assert(address == TEST_SETTINGS);
    return settings_memory;
}
#define MODULE_SETTINGS_MAP(address) mapped_settings(address)
/* The profile table of the harness, and a cycle counter that advances 100
   cycles at each read. */
#define TEST_PROFILE 0x5d000000U
static uint32_t profile_memory[GRKIT_PROFILE_SIZE / 4], profile_clock;
static volatile uint32_t *mapped_profile(uint32_t address)
{
    assert(address >= TEST_PROFILE && address < TEST_PROFILE + sizeof(profile_memory) && (address & 3) == 0);
    return profile_memory + (address - TEST_PROFILE) / 4;
}
#define PROFILE_MAP(address) mapped_profile(address)
#define PROFILE_START() (profile_clock += 100)
#define PROFILE_NOW() (profile_clock += 100)
static uint32_t profile_core_id;
#define PROFILE_CORE() profile_core_id
/* Module layers: the harness maps each paint address to a function of the
   Framing (modules/framing) and Guides (modules/guides) modules. */
#define TEST_MASK_PAINT 0x5e000011U
#define TEST_GUIDES_PAINT 0x5e000001U
#define TEST_GUIDES_ALWAYS_PAINT 0x5e000031U
#define TEST_BADGE_PAINT 0x5e000021U
static void test_module_paint(uint32_t address, const void *surface, uint32_t value);
#define MODULE_PAINT(address,surface,value) test_module_paint(address,surface,value)
static uint32_t level_updates, level_updated;
#define FRAMING_LEVEL_UPDATE(controller) (++level_updates, level_updated = (controller))
#include "framing.c"
#define GRKIT_PIXELS(address) mapped_pixels(address)
#include "../../modules/framing/framing.c"
#include "../../modules/guides/guides.c"
/* Styles 1 to 5 have an exact pixel model; 6, the golden spiral, and 7,
   dynamic symmetry, are checked to stay inside the area (the module test
   checks their lines). */
#define GUIDES_STYLE_MAX 7U
/* An ABI 2.1 label layer remains supported after built-ins use indicators. */
static void legacy_badge(const Surface *surface,u32 value) {
    (void)value;
    if(surface->area && !setting_get(SETTING_LABEL))
        overlay_label(surface,LABEL_RATIO(surface->area),LABEL_LEFT,LABEL_TOP);
}
static void test_module_paint(uint32_t address, const void *surface, uint32_t value)
{
    if (address == TEST_MASK_PAINT) framing_mask(surface, value);
    else if (address == TEST_GUIDES_ALWAYS_PAINT) guides_paint_always(surface, value);
    else if (address == TEST_BADGE_PAINT) legacy_badge(surface, value);
    else {
        assert(address == TEST_GUIDES_PAINT);
        guides_paint(surface, value);
    }
}
/* The committed Guides style. */
#define GUIDE_STYLE settings_memory[3]
#define GUIDE_DISP settings_memory[12]

static volatile u32 header[GRKIT_HOST_HEADER_SIZE / 4];
volatile u32 *framing_header(void) { return header; }
#define ASPECT_BIT (1U << (GRKIT_CORE_ASPECT - GRKIT_CORE_SETTING_BASE))
/* Register the module layers as INSTALL does, in painting order: with
   Framing, its mask (order 10) and its ratio label (order 30, full display,
   live view only) of CORE.ASPECT, around the Guides layer (order 20, DISP)
   of its STYLE setting (handle 0, 8 values). Without Framing, no module uses
   CORE.ASPECT. */
static void register_layers(u32 framing)
{
    const u32 layers[3][4] = {
        {TEST_MASK_PAINT, GRKIT_CORE_ASPECT, 10U << 16},
        {TEST_GUIDES_PAINT, 0, 20U << 16 | MODULE_LAYER_FOLLOW_DISP},
        {TEST_BADGE_PAINT, GRKIT_CORE_ASPECT, 30U << 16 | MODULE_LAYER_FULL_DISP | MODULE_LAYER_LIVE_ONLY},
    };
    u32 n = 0;
    for (u32 i = 0; i < 3; ++i) {
        if (!framing && i != 1) continue;
        for (u32 w = 0; w < 4; ++w) header[GRKIT_HOST_MODULE_LAYERS/4 + n*4 + w] = layers[i][w];
        ++n;
    }
    header[GRKIT_HOST_MODULE_LAYER_COUNT/4] = n;
    header[GRKIT_HOST_CAPABILITIES/4] = framing ? ASPECT_BIT : 0;
}
/* Register the two official Guides layers, including their mutually
   exclusive enabled conditions. Neither needs the Framing capability. */
static void register_conditional_guides(void)
{
    memset(layer_memory,0,sizeof(layer_memory));
    for(u32 i=0;i<2;++i) {
        u32 descriptor=TEST_LAYERS+i*48, condition=descriptor+28;
        WORD(descriptor,0)=28; WORD(descriptor,20)=1; WORD(descriptor,24)=condition;
        WORD(condition,0)=3; WORD(condition,4)=1; WORD(condition,8)=i;
        u32 *entry=(u32 *)MODULE_LAYER(header,i);
        entry[0]=i ? TEST_GUIDES_ALWAYS_PAINT : TEST_GUIDES_PAINT;
        entry[1]=0; entry[2]=20U<<16 | (i ? 0 : MODULE_LAYER_FOLLOW_DISP); entry[3]=descriptor;
    }
    header[GRKIT_HOST_MODULE_LAYER_COUNT/4]=2;
    header[GRKIT_HOST_CAPABILITIES/4]=0;
}
/* 7:6: a crop on every camera, and the highest ratio value. */
#define LAST_RATIO 7U
/* Off, and each ratio value that the target offers. The core crops each
   ratio, also one that the camera has natively. */
static u32 cropped(u32 value) { return value == 0 || framing_row(value); }
/* The capture aspect id of each crop value: 3:2, then 16:9 for XPan and
   16:9, 4:3, 16:9 for 2.35:1, and 4:3 for 1:1, 5:4, and 7:6. */
static const u32 capture_ids[] = {0, 2, 2, 1, 2, 1, 1, 1};

static void reset(void)
{
    memset((void *)header, 0, sizeof(header));
    memset(owner_memory, 0, sizeof(owner_memory));
    memset(canvas_memory, 0, sizeof(canvas_memory));
    memset(bitmap_memory, 0, sizeof(bitmap_memory));
    for (u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) pixels[i]=SENTINEL;
    WORD(TEST_OWNER, LIVEVIEW_CANVAS)=TEST_CANVAS;
    WORD(TEST_OWNER, 4)=TEST_CANVAS;
    WORD(TEST_CANVAS, 0x28)=720;
    WORD(TEST_CANVAS, 0x2c)=480;
    BYTE(TEST_CANVAS, 0x9e)=1;
    BYTE(TEST_BITMAP, 2)=5;
    HALF(TEST_BITMAP, 0x18)=TEST_STRIDE;
    HALF(TEST_BITMAP, 0x1a)=480;
    HALF(TEST_BITMAP, 0x1c)=720;
    HALF(TEST_BITMAP, 0x1e)=480;
    WORD(TEST_BITMAP, 4)=TEST_STRIDE*480*4;
    WORD(TEST_BITMAP, 8)=TEST_PIXELS;
    /* The Framing and Guides modules are loaded. */
    memset(settings_memory,0,sizeof(settings_memory));
    settings_memory[0]=5;
    for(u32 i=0;i<5;++i) { settings_memory[2+3*i]=i+1; settings_memory[4+3*i]=i==0 ? 8 : i==1 ? 3 : i==4 ? 4 : 2; }
    header[GRKIT_HOST_MODULE_SETTINGS/4]=TEST_SETTINGS;
    register_layers(1);
    header[GRKIT_HOST_ACTIVE/4]=0x59001000;
    header[GRKIT_HOST_RECORDS/4]=TEST_RECORDS;
    /* The tail holds the labels (runtime/abi.h: GRKIT_LABELS). */
    header[GRKIT_HOST_TAIL/4]=TEST_RECORDS;
    header[GRKIT_HOST_FRAME_RATIO/4]=1;
    native_aspect=0;
    display_mode=0;
    capture_aspect=0;
    capture_requests=0;
    framing_bind(TEST_OWNER);
    assert(header[GRKIT_HOST_FRAME_OWNER/4]==TEST_OWNER);
    assert(header[GRKIT_HOST_FRAME_CANVAS/4]==TEST_CANVAS);
}
static void assert_untouched(void)
{
    for (u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) assert(pixels[i]==SENTINEL);
    assert(header[GRKIT_HOST_FRAME_DRAWS/4]==0);
    assert(header[GRKIT_HOST_ACTIVE/4]==0x59001000);
}
static void assert_rejected(void)
{
    framing_paint(TEST_CANVAS, TEST_BITMAP);
    assert_untouched();
}
/* A field of the label table. */
static u32 table_field(u32 at,u32 bytes)
{
    u32 value=0;
    for(u32 i=0; i<bytes; ++i) value|=(u32)overlay_labels[at+i]<<(8*i);
    return value;
}
/* The pixel of label n at (x, y) when its top left is at (left, top): the
   canvas where the label is transparent, else an opaque gray. */
static u32 expected_label(u32 label,u32 left,u32 top,u32 x,u32 y,u32 background)
{
    u32 width=table_field(label*8,2), height=table_field(label*8+2,2), at=table_field(label*8+4,4), value=0;
    if(x<left || x>=left+width || y<top || y>=top+height) return background;
    /* The rows hold spans of drawn pixels: a count, then for each a u16 first
       column, a u16 length, and its pixels. */
    for(u32 row=0; row<=y-top; ++row) {
        u32 spans=overlay_labels[at++];
        for(u32 i=0; i<spans; ++i) {
            u32 first=table_field(at,2), length=table_field(at+2,2);
            if(row==y-top && x-left>=first && x-left<first+length) value=overlay_labels[at+4+x-left-first];
            at+=4+length;
        }
    }
    return value ? 0xff000000U|value*0x010101U : background;
}
static u32 expected_badge(u32 ratio,u32 x,u32 y,u32 background)
{
    return ratio ? expected_label(ratio,LABEL_LEFT,LABEL_TOP,x,y,background) : background;
}
/* Independent guide fixtures. Each value is a division, and its 2-pixel line
   covers the pixel before it and the pixel at it, as the native grid does.
   Thirds are at multiples of floor(span / 3). The golden section is rounded. */
/* By ratio value: 3:2 (Off), XPan, 16:9, 4:3, 2.35:1, 1:1, 5:4, 7:6. */
static const u32 boxes[][4]={{0,0,720,480},{0,107,720,373},{0,37,720,442},{40,0,680,480},
                             {0,87,720,393},{120,0,600,480},{60,0,660,480},{80,0,640,480}};
static const u32 thirds[][4]={{240,480,160,320},{240,480,195,283},{240,480,172,307},{253,466,160,320},
                              {240,480,189,291},{280,440,160,320},{260,460,160,320},{266,452,160,320}};
static const u32 golden[][4]={{275,445,183,297},{275,445,209,271},{275,445,192,287},{284,436,183,297},
                              {275,445,204,276},{303,417,183,297},{289,431,183,297},{294,426,183,297}};
/* The number of ratio values. It is also a value that no target offers. */
#define RATIO_VALUES 8U
static u32 on_line(u32 value,u32 line) { return value==line-1 || value==line; }
static u32 expected_guide(u32 ratio,u32 style,u32 x,u32 y)
{
    const u32 *box=boxes[ratio];
    if(x<box[0] || x>=box[2] || y<box[1] || y>=box[3]) return 0;
    /* Any pixel inside the area may be a curve or a reciprocal. */
    if(style==6 || style==7) return 2;
    if(style==1 || style==3 || style==5) {
        const u32 *lines=style==3 ? golden[ratio] : thirds[ratio];
        if(on_line(x,lines[0]) || on_line(x,lines[1]) || on_line(y,lines[2]) || on_line(y,lines[3])) return 1;
    }
    /* Every center is (360,240). Each arm reaches 20 pixels past the 2 center
       pixels 359..360 and 239..240. */
    if(style==2 || style==5)
        return (on_line(y,240) && x>=339 && x<=380) || (on_line(x,360) && y>=219 && y<=260);
    if(style==4) {
        /* The upper pixel of each pair goes from the top row to the row above
           the bottom one, rounded half up. */
        u32 width=box[2]-box[0], height=box[3]-box[1], column=x-box[0];
        u32 rows[2]={column, width-1-column};
        for(u32 i=0; i<2; ++i) {
            u32 upper=box[1]+(2*rows[i]*(height-2)+width-1)/(2*(width-1));
            if(y==upper || y==upper+1) return 1;
        }
    }
    return 0;
}
/* The load cue is centered in the row of the Framing label. */
static u32 expected_cue(u32 x,u32 y,u32 background)
{
    return expected_label(0,360-table_field(0,2)/2,LABEL_TOP,x,y,background);
}
/* Only the cue is on the canvas. */
static void assert_cue(void)
{
    for(u32 y=0; y<480; ++y) for(u32 x=0; x<TEST_STRIDE; ++x)
        assert(pixels[1+y*TEST_STRIDE+x]==(x<720 ? expected_cue(x,y,SENTINEL) : SENTINEL));
    assert(pixels[0]==SENTINEL);
}
/* A reset with no layer and a pending cue. */
static void reset_cue(u32 second)
{
    reset();
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_LOAD_CUE/4]=CUE_PENDING;
    local_second=second;
    invalidations=0;
    invalidated=0;
}
static void assert_overlay(u32 capture,u32 composition,u32 style,u32 badge)
{
    const u32 *box=boxes[composition];
    for(u32 y=0; y<480; ++y) for(u32 x=0; x<TEST_STRIDE; ++x) {
        u32 expected=SENTINEL;
        u32 guide=x<720 ? expected_guide(capture,style,x,y) : 0, line=0x80000000U;
        if(x<720) {
            if(composition && (x<box[0] || x>=box[2] || y<box[1] || y>=box[3])) expected=0x80000000;
            if(guide==1) expected=0x80000000U;
            if(badge) { expected=expected_badge(composition,x,y,expected); line=expected_badge(composition,x,y,line); }
        }
        assert(pixels[1+y*TEST_STRIDE+x]==expected || (guide==2 && pixels[1+y*TEST_STRIDE+x]==line));
    }
}
static void assert_guides(u32 ratio,u32 style,u32 badge)
{
    assert_overlay(ratio,ratio,style,badge);
}
static void guides_only(u32 enabled)
{
    header[GRKIT_HOST_CAPABILITIES/4] |= GRKIT_CORE_ASPECT_MODE_BIT;
    header[GRKIT_HOST_ASPECT_MODE/4]=enabled;
}
static void native_guides_follow_the_photo_area(void)
{
    static const u32 areas[]={0,3,2,5};
    for(u32 native=0;native<4;++native) for(u32 style=1;style<=GUIDES_STYLE_MAX;++style) {
        reset();
        register_layers(0);
        native_aspect=native; capture_aspect=native;
        GUIDE_STYLE=style;
        framing_bind(TEST_OWNER);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert_overlay(areas[native],0,style,0);
        assert(capture_aspect==native && header[GRKIT_HOST_FRAME_CROP/4]==0);
        assert(header[GRKIT_HOST_FRAME_DRAWS/4]==1);
        /* The Guides ADJ page uses the same native photo area. */
        for(u32 i=0;i<sizeof(pixels)/sizeof(pixels[0]);++i) pixels[i]=SENTINEL;
        header[GRKIT_HOST_FRAME_PREVIEW/4]=1;
        header[GRKIT_HOST_MODULE_PREVIEW/4]=1;
        header[GRKIT_HOST_MODULE_PREVIEW/4+1]=style;
        header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=TEST_OWNER;
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert_overlay(areas[native],0,style,0);
    }
    /* Native aspect changes redraw an existing canvas without a GRKit
       setting change, including erasing lines before an unknown aspect. */
    reset(); register_layers(0); GUIDE_STYLE=1;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    invalidations=0;
    native_aspect=3; capture_aspect=3;
    framing_bind(TEST_OWNER);
    assert(invalidations==1);
    framing_bind(TEST_OWNER);
    assert(invalidations==1);
    for(u32 i=0;i<sizeof(pixels)/sizeof(pixels[0]);++i) pixels[i]=SENTINEL;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_overlay(5,0,1,0);
    native_aspect=4;
    framing_bind(TEST_OWNER);
    assert(invalidations==2);
    for(u32 i=0;i<sizeof(pixels)/sizeof(pixels[0]);++i) pixels[i]=SENTINEL;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    for(u32 i=0;i<sizeof(pixels)/sizeof(pixels[0]);++i) assert(pixels[i]==SENTINEL);
}
static void guides_follow_composition_frames(void)
{
    for(u32 ratio=1;ratio<RATIO_VALUES;++ratio) {
        reset();
        guides_only(1);
        header[GRKIT_HOST_FRAME_RATIO/4]=ratio;
        GUIDE_STYLE=1;
        framing_bind(TEST_OWNER);
        assert(header[GRKIT_HOST_FRAME_RATIO/4]==ratio);
        assert(header[GRKIT_HOST_FRAME_CROP/4]==0 && capture_aspect==0);
        assert(framing_capture_request(0)==0 && framing_capture_request(3)==3);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        /* The capture stays uncropped, and Guides divide the composition frame. */
        assert_guides(ratio,1,1);
        /* Highlighting another ratio previews its frame with its Guides. */
        for(u32 i=0;i<sizeof(pixels)/sizeof(pixels[0]);++i) pixels[i]=SENTINEL;
        header[GRKIT_HOST_FRAME_PREVIEW/4]=3;
        header[GRKIT_HOST_MODULE_PREVIEW/4]=GRKIT_CORE_ASPECT+1;
        header[GRKIT_HOST_MODULE_PREVIEW/4+1]=2;
        header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=TEST_OWNER;
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert_guides(2,1,0);
        assert(header[GRKIT_HOST_FRAME_RATIO/4]==ratio && capture_aspect==0);
        assert(header[GRKIT_HOST_FRAME_CROP/4]==0);
    }
    /* Composition frames do not apply to native cropped views. Their
       Guides still draw within the actual photo, in live view and ADJ. */
    for(u32 native=1;native<4;++native) {
        static const u32 areas[]={0,3,2,5};
        reset(); guides_only(1);
        native_aspect=native; capture_aspect=native; GUIDE_STYLE=1;
        framing_bind(TEST_OWNER);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert_overlay(areas[native],0,1,0);
        assert(header[GRKIT_HOST_FRAME_CROP/4]==0 && capture_aspect==native);
        for(u32 i=0;i<sizeof(pixels)/sizeof(pixels[0]);++i) pixels[i]=SENTINEL;
        header[GRKIT_HOST_FRAME_PREVIEW/4]=2;
        header[GRKIT_HOST_MODULE_PREVIEW/4]=GRKIT_CORE_ASPECT+1;
        header[GRKIT_HOST_MODULE_PREVIEW/4+1]=1;
        header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=TEST_OWNER;
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert_overlay(areas[native],0,1,0);
    }
    /* Changing only the mode resends the capture request in both
       directions, without changing the stored composition ratio. */
    reset();
    assert(capture_aspect==2 && capture_requests==1);
    guides_only(1);
    framing_bind(TEST_OWNER);
    assert(capture_aspect==0 && capture_requests==2 && header[GRKIT_HOST_FRAME_CROP/4]==0);
    guides_only(0);
    framing_bind(TEST_OWNER);
    assert(capture_aspect==2 && capture_requests==3 && header[GRKIT_HOST_FRAME_CROP/4]==1);
    assert(header[GRKIT_HOST_FRAME_RATIO/4]==1);
}
/* The white of each record curve: R, G, B at input 255. */
static void record_white(u32 r, u32 g, u32 b)
{
    HALF(TEST_RECORD, GRKIT_RECORD_CURVES + 255 * 2) = (u16)r;
    HALF(TEST_RECORD, GRKIT_RECORD_CURVES + GRKIT_RECORD_CURVE_SIZE + 255 * 2) = (u16)g;
    HALF(TEST_RECORD, GRKIT_RECORD_CURVES + 2 * GRKIT_RECORD_CURVE_SIZE + 255 * 2) = (u16)b;
}
/* The live-view highlight warning: the threshold follows the white of the
   active effect, and each setting of the camera records its screen and mode. */
static void the_highlight_warning_follows_the_effect_white(void)
{
    reset();
    header[GRKIT_HOST_CAPABILITIES/4] |= GRKIT_CORE_TONE_BIT;
    /* Native processing keeps the native threshold, luma 254. */
    header[GRKIT_HOST_ACTIVE/4] = 0;
    assert(framing_level_value() == 0xfe00);
    /* A white of 13000 of 16383 is luma 203 (rounded up): the threshold is 202. */
    header[GRKIT_HOST_ACTIVE/4] = TEST_RECORD;
    record_white(13000, 13000, 13000);
    assert(framing_level_value() == 202 << 8);
    /* A full white keeps the native threshold. */
    record_white(16383, 16383, 16383);
    assert(framing_level_value() == 0xfe00);
    /* The luma weights: a red white weighs 77 of 256. */
    record_white(16383, 0, 0);
    assert(framing_level_value() == 76 << 8);
    /* The post matrix applies before the luma: half of each channel. */
    WORD(TEST_RECORD, GRKIT_RECORD_FEATURES) = GRKIT_FEATURE_POST;
    for (u32 i = 0; i < 9; ++i) HALF(TEST_RECORD, GRKIT_RECORD_POST + i * 2) = i % 4 ? 0 : 4096;
    record_white(16383, 16383, 16383);
    assert(framing_level_value() == 127 << 8);
    WORD(TEST_RECORD, GRKIT_RECORD_FEATURES) = 0;
    /* Without a module that uses CORE.TONE, the processing is native. */
    header[GRKIT_HOST_CAPABILITIES/4] &= ~GRKIT_CORE_TONE_BIT;
    record_white(13000, 13000, 13000);
    assert(framing_level_value() == 0xfe00);
    header[GRKIT_HOST_ACTIVE/4] = 0;
    framing_level_record(0x5c001000, 3);
    assert(header[GRKIT_HOST_LEVEL_SCREEN/4] == 0x5c001000 && header[GRKIT_HOST_LEVEL_MODE/4] == 3);
    framing_level_record(0x5c001000, 0);
    assert(header[GRKIT_HOST_LEVEL_MODE/4] == 0);
    /* The first live view update after INSTALL marks the warning as changed
       once, when the effect has its own threshold, so that the update sets
       the warning again. A setting then records the screen. */
    header[GRKIT_HOST_CAPABILITIES/4] |= GRKIT_CORE_TONE_BIT;
    header[GRKIT_HOST_ACTIVE/4] = TEST_RECORD;
    header[GRKIT_HOST_LEVEL_SCREEN/4] = 0;
    BYTE(TEST_OWNER, LIVEVIEW_LEVEL_PENDING) = 0;
    level_updates = 0;
    framing_bind(TEST_OWNER);
    assert(BYTE(TEST_OWNER, LIVEVIEW_LEVEL_PENDING) == 1 && header[GRKIT_HOST_LEVEL_SCREEN/4] == 1);
    assert(level_updates == 1 && level_updated == TEST_OWNER);
    BYTE(TEST_OWNER, LIVEVIEW_LEVEL_PENDING) = 0;
    framing_bind(TEST_OWNER);
    assert(BYTE(TEST_OWNER, LIVEVIEW_LEVEL_PENDING) == 0 && level_updates == 1);
    /* With the native threshold, nothing changes. */
    header[GRKIT_HOST_LEVEL_SCREEN/4] = 0;
    header[GRKIT_HOST_ACTIVE/4] = 0;
    framing_bind(TEST_OWNER);
    assert(BYTE(TEST_OWNER, LIVEVIEW_LEVEL_PENDING) == 0 && header[GRKIT_HOST_LEVEL_SCREEN/4] == 0);
    assert(level_updates == 1);
}
/* The live-view histogram: with an effect, the bins below the warning luma
   fill bins 0 to 126, and the last bin, the red clipping line, counts the
   luma that the highlight warning marks. */
static void the_histogram_follows_the_effect_white(void)
{
    u32 bins[128], i;
    reset();
    header[GRKIT_HOST_CAPABILITIES/4] |= GRKIT_CORE_TONE_BIT;
    for (i = 0; i < 128; ++i) bins[i] = i + 1;
    /* Native processing keeps the bins. */
    header[GRKIT_HOST_ACTIVE/4] = 0;
    framing_histogram_remap(bins);
    for (i = 0; i < 128; ++i) assert(bins[i] == i + 1);
    /* A full white keeps the bins. */
    header[GRKIT_HOST_ACTIVE/4] = TEST_RECORD;
    record_white(16383, 16383, 16383);
    framing_histogram_remap(bins);
    for (i = 0; i < 128; ++i) assert(bins[i] == i + 1);
    /* Warning luma 202: bins 101 to 127 hold luma 202 and up, and go to the
       last bin. Bin j below it takes bin j * 202 / 254. */
    record_white(13000, 13000, 13000);
    assert(framing_level_value() == 202 << 8);
    framing_histogram_remap(bins);
    assert(bins[127] == (102 + 128) * 27 / 2);
    assert(bins[0] == 1 && bins[1] == 1 && bins[2] == 2 && bins[64] == 51);
    assert(bins[126] == 100 + 1);
    for (i = 1; i < 127; ++i) assert(bins[i] >= bins[i - 1] && bins[i] <= 101);
    /* Without a module that uses CORE.TONE, the processing is native. */
    header[GRKIT_HOST_CAPABILITIES/4] &= ~GRKIT_CORE_TONE_BIT;
    for (i = 0; i < 128; ++i) bins[i] = 7;
    framing_histogram_remap(bins);
    for (i = 0; i < 128; ++i) assert(bins[i] == 7);
    header[GRKIT_HOST_ACTIVE/4] = 0;
}
/* Right-aligned status row: the last indicator ends at STATUS_RIGHT. */
#define STATUS_ONE (STATUS_RIGHT-40U)
#define STATUS_TWO (STATUS_RIGHT-STATUS_STEP-40U)
static void status_icons_follow_enable_strength_and_disp(void)
{
    reset();
    header[GRKIT_HOST_MODULE_LAYER_COUNT/4]=0;
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_INDICATOR_COUNT/4]=2;
    header[GRKIT_HOST_ICON_CHUNKS/4]=TEST_ICONS;
    settings_memory[0]=4;
    for(u32 i=0;i<4;++i) {
        settings_memory[SETTINGS_ENTRY(i)+1]=i%2 ? 0 : 1;
        settings_memory[SETTINGS_ENTRY(i)+2]=i%2 ? 3 : 2;
    }
    for(u32 i=0;i<2;++i) {
        u32 d=TEST_INDICATOR+i*128,c=d+32,icon=d+64;
        WORD(d,0)=24; WORD(d,4)=2; WORD(d,8)=0;
        WORD(d,12)=1; WORD(d,16)=c; WORD(d,20)=icon;
        WORD(c,0)=i*2; WORD(c,4)=1; WORD(c,8)=1;
        WORD(icon,0)=16; WORD(icon,4)=1; WORD(icon,8)=i*2+1; WORD(icon,12)=i*3;
        header[GRKIT_HOST_INDICATORS/4+i*2]=d;
        header[GRKIT_HOST_INDICATORS/4+i*2+1]=0;
    }
    memset(icon_memory,0,sizeof(icon_memory));
    for(u32 i=0;i<6;++i) {
        u32 icon=module_tile_address(header,GRKIT_ICON_MODULE_TILE(i));
        WORD(icon,GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE)=0xff000010U+i;
    }
    framing_bind(TEST_OWNER);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_TWO]==0xff000010U);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]==0xff000013U);
    /* Composition-only mode keeps status icons and leaves capture uncropped. */
    guides_only(1);
    header[GRKIT_HOST_FRAME_RATIO/4]=1;
    framing_bind(TEST_OWNER);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(header[GRKIT_HOST_FRAME_CROP/4]==0 && capture_aspect==0);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_TWO]==0xff000010U);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]==0xff000013U);
    guides_only(0);
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    framing_bind(TEST_OWNER);
    settings_memory[SETTINGS_ENTRY(1)+1]=2;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_TWO]==0xff000012U);
    settings_memory[SETTINGS_ENTRY(0)+1]=0;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]==0xff000013U);
    /* Screen status does not depend on the native photo area. */
    native_aspect=3;
    assert(status_shown(header));
    pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]=SENTINEL;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]==0xff000013U);
    for(display_mode=1;display_mode<=3;++display_mode) {
        assert(!status_shown(header));
        BYTE(TEST_CANVAS,0xa0)=0;
        /* The last bitmap still requests clearing after the icons hide. */
        header[GRKIT_HOST_FRAME_BITMAP/4]=TEST_BITMAP;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==1);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert(!header[GRKIT_HOST_FRAME_BITMAP/4]);
    }
    display_mode=0;
    settings_memory[SETTINGS_ENTRY(2)+1]=0;
    assert(!status_shown(header));
    /* Video cannot draw a status row. */
    settings_memory[SETTINGS_ENTRY(2)+1]=1;
    BYTE(TEST_OWNER,LIVEVIEW_MOVIE)=1;
    pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]=SENTINEL;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]==SENTINEL);
}
/* A core label (DATE) takes its own width and a gap in the status row, and
   the next icon follows it. */
static void status_row_lays_out_core_labels(void)
{
    reset();
    header[GRKIT_HOST_MODULE_LAYER_COUNT/4]=0;
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_INDICATOR_COUNT/4]=2;
    header[GRKIT_HOST_ICON_CHUNKS/4]=TEST_ICONS;
    settings_memory[0]=4;
    for(u32 i=0;i<4;++i) {
        settings_memory[SETTINGS_ENTRY(i)+1]=i%2 ? 0 : 1;
        settings_memory[SETTINGS_ENTRY(i)+2]=i%2 ? 3 : 2;
    }
    for(u32 i=0;i<2;++i) {
        u32 d=TEST_INDICATOR+i*128,c=d+32,icon=d+64;
        WORD(d,0)=24; WORD(d,4)=2; WORD(d,8)=0;
        WORD(d,12)=1; WORD(d,16)=c; WORD(d,20)=icon;
        WORD(c,0)=i*2; WORD(c,4)=1; WORD(c,8)=1;
        WORD(icon,0)=16; WORD(icon,4)=1; WORD(icon,8)=i*2+1; WORD(icon,12)=i*3;
        header[GRKIT_HOST_INDICATORS/4+i*2]=d;
        header[GRKIT_HOST_INDICATORS/4+i*2+1]=0;
    }
    /* Indicator 0 shows the core DATE label. */
    WORD(TEST_INDICATOR+64,4)=GRKIT_ICON_LABEL; WORD(TEST_INDICATOR+64,8)=0xffffffffU;
    WORD(TEST_INDICATOR+64,12)=LABEL_DATE;
    memset(icon_memory,0,sizeof(icon_memory));
    for(u32 i=0;i<6;++i) {
        u32 icon=module_tile_address(header,GRKIT_ICON_MODULE_TILE(i));
        WORD(icon,GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE)=0xff000010U+i;
    }
    framing_bind(TEST_OWNER);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    u32 width=label_width(LABEL_DATE),height=label_field(overlay_labels,LABEL_DATE*8+2,2),drawn=0;
    assert(width>40 && height<40);
    /* The row ends at STATUS_RIGHT: the label, its gap, then the icon. */
    u32 left=STATUS_RIGHT-width-STATUS_LABEL_GAP-40;
    for(u32 y=0;y<height;++y)
        for(u32 x=0;x<width;++x)
            drawn+=pixels[1+(LABEL_TOP+y)*TEST_STRIDE+left+x]!=SENTINEL;
    assert(drawn>0);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+left+width+STATUS_LABEL_GAP]==0xff000013U);
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+left+STATUS_STEP]!=0xff000013U);
    for(u32 x=0;x<left;++x) assert(pixels[1+STATUS_TOP*TEST_STRIDE+x]==SENTINEL);
    /* The row waits while the load cue shows, and returns after it. */
    local_second=100;
    header[GRKIT_HOST_LOAD_CUE/4]=local_second+2;
    pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]=SENTINEL;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(cue_shown(header));
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]==SENTINEL);
    local_second+=CUE_SECONDS;
    framing_bind(TEST_OWNER);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(!cue_shown(header));
    assert(pixels[1+STATUS_TOP*TEST_STRIDE+STATUS_ONE]==0xff000013U);
}
/* Native view `i` at x, y, width, height, visible, without children. */
static u32 test_view(u32 i,int x,int y,int w,int h,u32 visible)
{
    u32 view=TEST_VIEWS+i*TEST_VIEW_SIZE;
    memset((uint8_t *)view_memory+i*TEST_VIEW_SIZE,0,TEST_VIEW_SIZE);
    WORD(view,VIEW_SCREEN)=(u32)x; WORD(view,VIEW_SCREEN+4)=(u32)y;
    WORD(view,VIEW_SCREEN+8)=(u32)w; WORD(view,VIEW_SCREEN+12)=(u32)h;
    BYTE(view,VIEW_VISIBLE)=(uint8_t)visible;
    return view;
}
/* The children of `parent`: the views in `views`, in a vector at `vector`. */
static void test_children(u32 parent,u32 vector,const u32 *views,u32 count)
{
    for(u32 i=0;i<count;++i) WORD(vector,i*4)=views[i];
    WORD(parent,VIEW_CHILDREN)=vector;
    WORD(parent,VIEW_CHILDREN+4)=vector+count*4;
}
/* The first column of the status row where `color` shows, or 720. */
static u32 status_column(u32 color)
{
    for(u32 x=0;x<720;++x) if(pixels[1+STATUS_TOP*TEST_STRIDE+x]==color) return x;
    return 720;
}
static void status_paint_again(void)
{
    for (u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) pixels[i]=SENTINEL;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
}
/* The row takes the rightmost free part of the second row, STATUS_GAP from
   each visible native view: the focus distance (600) and the dial guide of
   GR IV Snap mode (440 to 600). A hidden view, the children of a hidden
   view, a view that crosses the row, and a view as wide as the screen leave
   the row free. When no free
   part holds every indicator, the widest one holds the first ones. */
static void status_row_avoids_native_views(void)
{
    reset();
    header[GRKIT_HOST_MODULE_LAYER_COUNT/4]=0;
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_INDICATOR_COUNT/4]=2;
    header[GRKIT_HOST_ICON_CHUNKS/4]=TEST_ICONS;
    settings_memory[0]=4;
    for(u32 i=0;i<4;++i) {
        settings_memory[SETTINGS_ENTRY(i)+1]=i%2 ? 0 : 1;
        settings_memory[SETTINGS_ENTRY(i)+2]=i%2 ? 3 : 2;
    }
    for(u32 i=0;i<2;++i) {
        u32 d=TEST_INDICATOR+i*128,c=d+32,icon=d+64;
        WORD(d,0)=24; WORD(d,4)=2; WORD(d,8)=0;
        WORD(d,12)=1; WORD(d,16)=c; WORD(d,20)=icon;
        WORD(c,0)=i*2; WORD(c,4)=1; WORD(c,8)=1;
        WORD(icon,0)=16; WORD(icon,4)=1; WORD(icon,8)=i*2+1; WORD(icon,12)=i*3;
        header[GRKIT_HOST_INDICATORS/4+i*2]=d;
        header[GRKIT_HOST_INDICATORS/4+i*2+1]=0;
    }
    memset(icon_memory,0,sizeof(icon_memory));
    for(u32 i=0;i<6;++i) {
        u32 icon=module_tile_address(header,GRKIT_ICON_MODULE_TILE(i));
        WORD(icon,GRKIT_ICON_STRIDE-GRKIT_GRF_ICON_SIZE)=0xff000010U+i;
    }
    /* The canvas: a full-screen parent, the focus distance, a hidden icon at
       the right, and the Snap dial guide (a parent of two views, and one
       more view). */
    u32 full=test_view(0,0,0,720,480,1),distance=test_view(1,600,40,80,40,1),hidden=test_view(2,680,40,40,40,0);
    u32 guide=test_view(3,440,40,120,40,1),dial=test_view(4,440,40,40,40,1),value=test_view(5,480,40,80,40,1);
    u32 rear=test_view(6,560,40,40,40,1),shift=test_view(7,100,40,250,20,0);
    /* A focus frame at the top of the photo crosses the row. */
    u32 focus=test_view(8,328,60,64,52,1);
    u32 top[]={full,distance,hidden,guide,rear,shift,focus},parts[]={dial,value};
    test_children(guide,TEST_VECTORS,parts,2);
    test_children(TEST_CANVAS,TEST_VECTORS+0x20,top,7);
    BYTE(guide,VIEW_VISIBLE)=0; BYTE(rear,VIEW_VISIBLE)=0;
    framing_bind(TEST_OWNER);
    /* The focus distance alone: the row ends at STATUS_RIGHT, 20 columns
       before it, as without native views. The full-screen view, the hidden
       icon at the right, the children of the hidden guide, and the focus
       frame that crosses the row do not count. */
    status_paint_again();
    assert(status_column(0xff000013U)==STATUS_RIGHT-40);
    assert(status_column(0xff000010U)==STATUS_RIGHT-STATUS_STEP-40);
    /* Snap mode: the row ends STATUS_GAP before the dial guide. */
    BYTE(guide,VIEW_VISIBLE)=1; BYTE(rear,VIEW_VISIBLE)=1;
    status_paint_again();
    assert(status_column(0xff000013U)==440-STATUS_GAP-40);
    assert(status_column(0xff000010U)==440-STATUS_GAP-STATUS_STEP-40);
    /* A wide native view leaves 7 to 80 and 370 to 420. The widest part
       holds only the first icon, at its right end. */
    BYTE(shift,VIEW_VISIBLE)=1;
    status_paint_again();
    assert(status_column(0xff000010U)==100-STATUS_GAP-40);
    assert(status_column(0xff000013U)==720);
    /* No free part holds an icon: no row. */
    WORD(shift,VIEW_SCREEN)=0; WORD(shift,VIEW_SCREEN+8)=700;
    status_paint_again();
    assert(status_column(0xff000010U)==720 && status_column(0xff000013U)==720);
}
/* The widest set of the official indicators fits between STATUS_LEFT and
   STATUS_RIGHT: a Framing ratio, a Date Imprint format, and the Bloom,
   Halation, and Grain icons. */
static void status_row_fits_the_official_indicators(void)
{
    u32 ratio=0,date=0;
    reset();
    for(u32 label=1;label<LABEL_DATE;++label)
        if(label_width(label)>ratio) ratio=label_width(label);
    for(u32 style=1;LABEL_DATE_FORMAT(style)<GRKIT_LABEL_COUNT;++style)
        if(label_width(LABEL_DATE_FORMAT(style))>date) date=label_width(LABEL_DATE_FORMAT(style));
    assert(ratio>0 && date>0);
    /* One-line labels sit at LABEL_TOP; the two-line formats (with the time)
       center like the icons from STATUS_TOP. */
    assert(label_field(overlay_labels,LABEL_DATE*8+2,2)<=STATUS_LINE_HEIGHT);
    assert(label_field(overlay_labels,LABEL_DATE_FORMAT(2)*8+2,2)>STATUS_LINE_HEIGHT);
    assert(label_field(overlay_labels,LABEL_DATE_FORMAT(2)*8+2,2)<=40);
    assert(ratio+STATUS_LABEL_GAP+date+STATUS_LABEL_GAP+2*STATUS_STEP+40<=STATUS_RIGHT-STATUS_LEFT);
}

int main(void)
{
    native_guides_follow_the_photo_area();
    guides_follow_composition_frames();
    status_icons_follow_enable_strength_and_disp();
    status_row_lays_out_core_labels();
    status_row_fits_the_official_indicators();
    status_row_avoids_native_views();
    the_highlight_warning_follows_the_effect_white();
    the_histogram_follows_the_effect_white();
    /* Fixed rectangles are independent expected output, not a second renderer. */
    const u32 rectangles[][4]={{0,0,720,480},{0,107,720,373},{0,37,720,442},{40,0,680,480},
                               {0,87,720,393},{120,0,600,480},{60,0,660,480},{80,0,640,480}};
    for (u32 ratio=0; ratio<RATIO_VALUES; ++ratio) {
        if(!cropped(ratio)) continue;
        reset();
        header[GRKIT_HOST_FRAME_RATIO/4]=ratio;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==(ratio!=0));
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        const u32 *box=rectangles[ratio];
        for(u32 y=0; y<480; ++y) for(u32 x=0; x<TEST_STRIDE; ++x) {
            u32 expected=SENTINEL;
            if(ratio && x<720 && (x<box[0] || x>=box[2] || y<box[1] || y>=box[3])) expected=0x80000000;
            expected=expected_badge(ratio,x,y,expected);
            assert(pixels[1+y*TEST_STRIDE+x]==expected);
        }
        for(u32 i=1+TEST_STRIDE*480; i<sizeof(pixels)/sizeof(pixels[0]); ++i) assert(pixels[i]==SENTINEL);
        assert(pixels[0]==SENTINEL);
        assert(header[GRKIT_HOST_FRAME_DRAWS/4]==(ratio!=0));
        assert(header[GRKIT_HOST_ACTIVE/4]==0x59001000);
    }
    /* Without a module that uses CORE.ASPECT, a stored crop does nothing,
       and the guides use the full 3:2 area. */
    reset();
    header[GRKIT_HOST_FRAME_RATIO/4]=2;
    GUIDE_STYLE=1;
    register_layers(0);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_guides(0,1,0);
    assert(framing_capture_request(0)==0);
    /* Without module layers, nothing paints. */
    reset();
    GUIDE_STYLE=1;
    header[GRKIT_HOST_MODULE_LAYER_COUNT/4]=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==0);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    /* Each draw requests the native full clear, which removes the previous
       label. A label keeps the mask under its transparent pixels. A target
       paints only the ratios that it offers. */
    reset();
    const u32 transitions[]={1,2,3,4,5,6,1,3,6,2,5,4};
    for(u32 i=0, draws=0; i<sizeof(transitions)/sizeof(transitions[0]); ++i) {
        u32 ratio=transitions[i];
        if(!cropped(ratio)) continue;
        header[GRKIT_HOST_FRAME_RATIO/4]=ratio;
        BYTE(TEST_CANVAS,0xa0)=0;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==1);
        for(u32 j=1; j<=TEST_STRIDE*480; ++j) pixels[j]=0;
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        const u32 *box=boxes[ratio];
        for(u32 y=0; y<120; ++y) for(u32 x=0; x<720; ++x) {
            u32 masked=x<box[0] || x>=box[2] || y<box[1] || y>=box[3];
            assert(pixels[1+y*TEST_STRIDE+x]==expected_badge(ratio,x,y,masked ? 0x80000000U : 0));
        }
        assert(header[GRKIT_HOST_FRAME_DRAWS/4]==++draws);
    }
    /* The ADJ preview canvas shows the highlighted ratio of the Framing page
       (runtime/modules.h: frame_preview) with its mask, without a label. */
    reset();
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_FRAME_PREVIEW/4]=3;
    header[GRKIT_HOST_MODULE_PREVIEW/4]=GRKIT_CORE_ASPECT+1; header[GRKIT_HOST_MODULE_PREVIEW/4+1]=2;
    header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=TEST_OWNER;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==1);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    for(u32 y=0; y<480; ++y) for(u32 x=0; x<TEST_STRIDE; ++x) {
        u32 masked=x<720 && (y<37 || y>=442);
        assert(pixels[1+y*TEST_STRIDE+x]==(masked ? 0x80000000 : SENTINEL));
    }
    assert(header[GRKIT_HOST_FRAME_DRAWS/4]==0);
    /* Off previews no mask but keeps forcing full draws. */
    header[GRKIT_HOST_FRAME_PREVIEW/4]=1;
    header[GRKIT_HOST_MODULE_PREVIEW/4+1]=0;
    for(u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) pixels[i]=SENTINEL;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    assert(header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]==TEST_OWNER);
    /* An ended preview gets one more full draw, then releases the canvas. */
    header[GRKIT_HOST_FRAME_PREVIEW/4]=0;
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==1);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]==0);
    assert_untouched();
    /* ADJ that opens on GRKit refreshes before it makes its canvas. The
       preview names the controller, so the first draw of the canvas that
       the controller gets shows the preview. */
    reset();
    header[GRKIT_HOST_FRAME_OWNER/4]=0; header[GRKIT_HOST_FRAME_CANVAS/4]=0;
    header[GRKIT_HOST_FRAME_PREVIEW/4]=2;
    WORD(TEST_OWNER,4)=0;
    header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=TEST_OWNER;
    WORD(TEST_OWNER,4)=TEST_CANVAS;
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==1);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(pixels[1]==0x80000000 && pixels[1+240*TEST_STRIDE]==SENTINEL);
    /* Unsupported bitmaps stay untouched in the preview path too. */
    reset();
    header[GRKIT_HOST_FRAME_PREVIEW/4]=2;
    header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=TEST_OWNER;
    BYTE(TEST_BITMAP,2)=4;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    /* Guides stay inside the photo area of each ratio, below the badge. */
    for (u32 ratio=0; ratio<RATIO_VALUES; ++ratio) for (u32 style=1; style<=GUIDES_STYLE_MAX; ++style) {
        if(!cropped(ratio)) continue;
        reset();
        header[GRKIT_HOST_FRAME_RATIO/4]=ratio;
        GUIDE_STYLE=style;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==1);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert_guides(ratio,style,1);
        assert(header[GRKIT_HOST_FRAME_DRAWS/4]==1);
        assert(header[GRKIT_HOST_FRAME_BITMAP/4]==TEST_BITMAP);
    }
    /* The diagonals run corner to corner. */
    for (u32 ratio=0; ratio<RATIO_VALUES; ++ratio) {
        const u32 *box=boxes[ratio];
        assert(expected_guide(ratio,4,box[0],box[1]) && expected_guide(ratio,4,box[2]-1,box[3]-1));
        assert(expected_guide(ratio,4,box[2]-1,box[1]) && expected_guide(ratio,4,box[0],box[3]-1));
    }
    /* Turning the guides off requests one more full clear. */
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    GUIDE_STYLE=0;
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==1);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(header[GRKIT_HOST_FRAME_BITMAP/4]==0);
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==0);
    /* DISP full and simple show the guides. No information and display off
       hide them, keep the Framing mask, and clear the old drawing once. */
    for (u32 mode=0; mode<4; ++mode) {
        reset();
        header[GRKIT_HOST_FRAME_RATIO/4]=0;
        GUIDE_STYLE=1;
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        display_mode=mode;
        BYTE(TEST_CANVAS,0xa0)=0;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==1);
        for(u32 i=1; i<=TEST_STRIDE*480; ++i) pixels[i]=SENTINEL;
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        if(mode<2) assert_guides(0,1,0);
        else {
            for(u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) assert(pixels[i]==SENTINEL);
            assert(header[GRKIT_HOST_FRAME_BITMAP/4]==0);
            BYTE(TEST_CANVAS,0xa0)=0;
            framing_prepare(TEST_CANVAS);
            assert(BYTE(TEST_CANVAS,0xa0)==0);
        }
    }
    /* The Framing badge shows only with the full display, like the top row of
       native settings. The mask stays in each display. */
    for (u32 mode=0; mode<4; ++mode) {
        reset();
        header[GRKIT_HOST_FRAME_RATIO/4]=LAST_RATIO;
        GUIDE_STYLE=1;
        display_mode=mode;
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert_guides(LAST_RATIO,mode<2,mode==0);
    }
    /* Unknown styles draw nothing. */
    reset();
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    GUIDE_STYLE=GUIDES_STYLE_MAX+1;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==0);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    /* The ADJ preview draws guides without a badge, with or without a ratio. */
    reset();
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_FRAME_PREVIEW/4]=1;
    header[GRKIT_HOST_MODULE_PREVIEW/4]=1; header[GRKIT_HOST_MODULE_PREVIEW/4+1]=1;
    header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=TEST_OWNER;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_guides(0,1,0);
    /* The Guides page with the committed XPan crop, which each camera
       crops. */
    for(u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) pixels[i]=SENTINEL;
    header[GRKIT_HOST_FRAME_RATIO/4]=1;
    header[GRKIT_HOST_FRAME_PREVIEW/4]=2;
    header[GRKIT_HOST_MODULE_PREVIEW/4]=1; header[GRKIT_HOST_MODULE_PREVIEW/4+1]=4;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_guides(1,4,0);
    assert(header[GRKIT_HOST_FRAME_DRAWS/4]==0);
    for(u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) pixels[i]=SENTINEL;
    /* A highlighted value out of range paints that layer at Off; the mask
       stays. */
    header[GRKIT_HOST_MODULE_PREVIEW/4]=1; header[GRKIT_HOST_MODULE_PREVIEW/4+1]=GUIDES_STYLE_MAX+1;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_guides(1,0,0);
    assert(header[GRKIT_HOST_FRAME_DRAWS/4]==0);
    /* Native 3:2 requests record the Framing ratio; other native ratios pass.
       XPan and 2.35:1 use id 2. */
    {
        for(u32 ratio=0; ratio<RATIO_VALUES; ++ratio) {
            if(!cropped(ratio)) continue;
            header[GRKIT_HOST_FRAME_RATIO/4]=ratio;
            assert(framing_capture_request(0)==capture_ids[ratio]);
            assert(framing_capture_request(3)==3);
            assert(framing_capture_request(1)==1);
        }
        header[GRKIT_HOST_FRAME_RATIO/4]=RATIO_VALUES;
        assert(framing_capture_request(0)==0);
        header[GRKIT_HOST_FRAME_RATIO/4]=999;
        assert(framing_capture_request(0)==0);
    }
    /* A selection that has not drawn yet requests one redraw, as after a restore at power-on. */
    reset();
    assert(invalidations >= 1 && invalidated == TEST_CANVAS);
    invalidations = 0;
    framing_bind(TEST_OWNER);
    assert(invalidations == 1);
    framing_paint(TEST_CANVAS, TEST_BITMAP);
    framing_bind(TEST_OWNER);
    assert(invalidations == 1);
    header[GRKIT_HOST_FRAME_RATIO/4] = 0;
    header[GRKIT_HOST_FRAME_BITMAP/4] = 0;
    framing_bind(TEST_OWNER);
    assert(invalidations == 1);
    GUIDE_STYLE=2;
    framing_bind(TEST_OWNER);
    assert(invalidations == 2);
    /* A setting that changed since the last redraw, for example on a native
       settings page, redraws a canvas that has drawn once. */
    framing_paint(TEST_CANVAS, TEST_BITMAP);
    framing_bind(TEST_OWNER);
    assert(invalidations == 2);
    header[GRKIT_HOST_FRAME_RATIO/4] = 5;
    ++header[GRKIT_HOST_STATE_CHANGES/4];
    framing_bind(TEST_OWNER);
    assert(invalidations == 3 && invalidated == TEST_CANVAS);
    framing_bind(TEST_OWNER);
    assert(invalidations == 3);
    /* Off redraws too, so the next draw erases the mask. */
    header[GRKIT_HOST_FRAME_RATIO/4] = 0; GUIDE_STYLE=0;
    ++header[GRKIT_HOST_STATE_CHANGES/4];
    framing_bind(TEST_OWNER);
    assert(invalidations == 4);
    /* Returning to the live view ends the playback 65:24 variant. */
    reset();
    ((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]=2;
    framing_bind(TEST_OWNER);
    assert(((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]==0);
#ifdef DEVELOP_SQUARE
    /* RAW development of the DNG in playback: the initial aspect and
       GRKIT_HOST_FRAME_FILE follow the size of its photo. A Framing size
       develops as recorded (0), a Framing 1:1 as the native 1:1, a native
       aspect natively (-1). */
    {
        static const u16 xpan[]={XPAN_SIZES}, scope[]={SCOPE_SIZES}, narrow[]={NARROW_SIZES};
        static const u16 wide_stock[]={XPAN_HEIGHT_0};
        const struct { u32 width, height, aspect, value, file; } cases[]={
            {xpan[0],xpan[1],2,0,2}, {xpan[1],xpan[0],2,0,2}, {scope[2],scope[3],2,0,3},
            {narrow[0],narrow[1],1,0,4}, {narrow[8],narrow[9],1,0,5},
            {narrow[2],narrow[2],1,DEVELOP_SQUARE,1},
            {xpan[0],xpan[0]*2/3,0,0xffffffffU,1},
            {xpan[0],wide_stock[0],2,DEVELOP_NATIVE_ASPECTS>>2&1 ? 0xffffffffU : 0,1},
        };
        for (u32 i = 0; i < sizeof(cases)/sizeof(cases[0]); ++i) {
            HALF(TEST_METADATA,METADATA_SIZE)=(u16)cases[i].width;
            HALF(TEST_METADATA,METADATA_SIZE+4)=(u16)cases[i].height;
            ((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]=0;
            assert(framing_develop_select(TEST_OWNER,cases[i].aspect)==cases[i].value);
            assert(((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]==cases[i].file);
        }
        /* An aspect that the user selects takes the stock sizes, the recorded
           aspect (0) the geometry of the DNG again. */
        HALF(TEST_METADATA,METADATA_SIZE)=xpan[0];
        HALF(TEST_METADATA,METADATA_SIZE+4)=xpan[1];
        framing_develop_geometry(TEST_OWNER,1);
        assert(((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]==1);
        framing_develop_geometry(TEST_OWNER,0);
        assert(((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]==2);
        /* A Framing ratio of the aspect list takes the capture aspect and
           the geometry of the ratio, whatever native value the model keeps. */
        {
            const struct { u32 ratio, capture, file; } ratios[]={
                {1,2,2}, {4,2,3}, {2,2,1}, {3,1,1}, {6,1,4}, {7,1,5},
            };
            for (u32 i = 0; i < sizeof(ratios)/sizeof(ratios[0]); ++i) {
                header[GRKIT_HOST_DEVELOP_RATIO/4]=ratios[i].ratio;
                framing_develop_geometry(TEST_OWNER,1);
                assert(((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]==ratios[i].file);
                assert(framing_develop_capture(0)==ratios[i].capture);
            }
            /* The native aspect keeps the native conversion. */
            header[GRKIT_HOST_DEVELOP_RATIO/4]=0;
            assert(framing_develop_capture(3)==3);
            framing_develop_geometry(TEST_OWNER,1);
            assert(((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]==1);
        }
        ((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]=0;
    }
    /* The ratio rows of the aspect list, by the capture aspects of its native
       rows: every crop with 3:2, only the crops of the capture aspect without
       it, never a ratio with a native row of its stock geometry. */
    {
        const struct { u32 captures; u8 rows[8]; } lists[]={
            {1U<<0|1U<<3, {1,4,2,3,6,7,0}},
            {1U<<0|1U<<1|1U<<2|1U<<3, {1,4,6,7,0}},
            {1U<<2, {1,4,0}},
            {1U<<1|1U<<3, {6,7,0}},
            {1U<<3, {0}},
        };
        for (u32 i = 0; i < sizeof(lists)/sizeof(lists[0]); ++i) {
            u32 n = 0;
            while (lists[i].rows[n]) {
                assert(develop_ratio_row(lists[i].captures,n)==lists[i].rows[n]);
                ++n;
            }
            assert(develop_ratio_row(lists[i].captures,n)==0);
            assert(develop_ratio_rows(lists[i].captures)==n);
        }
    }
#endif
    /* Live-view updates resend the 3:2 request only when capture differs. */
    reset();
    assert(capture_requests==1 && capture_aspect==2);
    framing_bind(TEST_OWNER);
    assert(capture_requests==1);
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    framing_bind(TEST_OWNER);
    assert(capture_requests==2 && capture_aspect==0);
    header[GRKIT_HOST_FRAME_RATIO/4]=LAST_RATIO;
    framing_bind(TEST_OWNER);
    assert(capture_requests==3 && capture_aspect==capture_ids[LAST_RATIO]);
    /* The capture sizes follow the crop in the native 3:2. */
    assert(header[GRKIT_HOST_FRAME_CROP/4]==LAST_RATIO);
    header[GRKIT_HOST_FRAME_RATIO/4]=1;
    native_aspect=3; capture_aspect=3;
    framing_bind(TEST_OWNER);
    assert(capture_requests==3 && capture_aspect==3);
    /* A native aspect from the camera menu keeps its stock sizes, so a
       saved crop does not crop its photos. GR IV crops a native 16:9 photo
       to XPan and 2.35:1 only. */
    assert(header[GRKIT_HOST_FRAME_CROP/4]==0);
    for(uint32_t native=1;native<=3;++native) {
        native_aspect=native; capture_aspect=native;
        for(uint32_t ratio=1;ratio<=LAST_RATIO;++ratio) {
            header[GRKIT_HOST_FRAME_RATIO/4]=ratio;
            framing_bind(TEST_OWNER);
            assert(header[GRKIT_HOST_FRAME_CROP/4]==(framing_row(ratio) && FRAMING_CROP_KEEPS(native,ratio) ? ratio : 0));
        }
    }
    header[GRKIT_HOST_FRAME_RATIO/4]=1;
    native_aspect=3; capture_aspect=3;
    native_aspect=0; capture_aspect=0;
    BYTE(TEST_OWNER,LIVEVIEW_MOVIE)=1;
    framing_bind(TEST_OWNER);
    assert(capture_requests==3);
    BYTE(TEST_OWNER,LIVEVIEW_MOVIE)=0;
    framing_bind(TEST_OWNER);
    assert(capture_requests==4 && capture_aspect==2);
    reset(); WORD(TEST_OWNER,4)=TEST_CANVAS+4; assert_rejected();
    reset(); BYTE(TEST_OWNER,LIVEVIEW_MOVIE)=1; assert_rejected();
    reset(); BYTE(TEST_CANVAS,0x9e)=0; assert_rejected();
    reset(); WORD(TEST_CANVAS,0x20)=1; assert_rejected();
    reset(); WORD(TEST_CANVAS,0x24)=1; assert_rejected();
    reset(); WORD(TEST_CANVAS,0x28)=721; assert_rejected();
    reset(); WORD(TEST_CANVAS,0x2c)=481; assert_rejected();
    reset(); BYTE(TEST_BITMAP,2)=4; assert_rejected();
    reset(); HALF(TEST_BITMAP,0x1c)=719; assert_rejected();
    reset(); HALF(TEST_BITMAP,0x1e)=479; assert_rejected();
    reset(); HALF(TEST_BITMAP,0x18)=719; assert_rejected();
    reset(); HALF(TEST_BITMAP,0x18)=1025; assert_rejected();
    reset(); HALF(TEST_BITMAP,0x1a)=479; assert_rejected();
    reset(); WORD(TEST_BITMAP,8)=0x3ffffffc; assert_rejected();
    reset(); WORD(TEST_BITMAP,8)=0x80000000-TEST_STRIDE*480*4+4; assert_rejected();
    reset(); WORD(TEST_BITMAP,8)=TEST_PIXELS+1; assert_rejected();
    reset(); WORD(TEST_BITMAP,4)=TEST_STRIDE*480*4-1; assert_rejected();
    /* Native 1:1 has its own guide area and no Framing mask. */
    reset(); native_aspect=3; GUIDE_STYLE=1;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_overlay(5,0,1,0);
#ifdef FRAMING_XPAN_NATIVE
    /* GR IV: XPan also crops native FRAMING_XPAN_NATIVE photos, so its mask
       shows there. Without that crop, Guides use the native 16:9 area. */
    reset(); native_aspect=FRAMING_XPAN_NATIVE;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(header[GRKIT_HOST_FRAME_DRAWS/4]==1 && pixels[1]==0x80000000);
    reset(); native_aspect=FRAMING_XPAN_NATIVE;
    header[GRKIT_HOST_FRAME_RATIO/4]=0; GUIDE_STYLE=1;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_overlay(2,0,1,0);
#else
    /* GR III: a native ratio does not activate the stored Framing ratio. */
    reset(); native_aspect=2; GUIDE_STYLE=1;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_overlay(2,0,1,0);
#endif
    reset(); native_aspect=4; GUIDE_STYLE=1; assert_rejected();
    reset(); header[GRKIT_HOST_FRAME_RATIO/4]=RATIO_VALUES;
    framing_paint(TEST_CANVAS,TEST_BITMAP); assert_untouched();
    reset(); framing_paint(TEST_CANVAS+4,TEST_BITMAP); assert_untouched();
    reset(); header[GRKIT_HOST_FRAME_OWNER/4]=0;
    framing_paint(TEST_CANVAS,TEST_BITMAP); assert_untouched();

    /* Off requests native background clearing once, then stops forcing draws. */
    reset();
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(header[GRKIT_HOST_FRAME_BITMAP/4]==TEST_BITMAP);
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==1);
    /* Simulate the native clear between Prepare and Paint. */
    for(u32 i=1; i<=TEST_STRIDE*480; ++i) pixels[i]=0;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(header[GRKIT_HOST_FRAME_BITMAP/4]==0);
    for(u32 i=1; i<=TEST_STRIDE*480; ++i) assert(pixels[i]==0);
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==0);

    reset();
    framing_leave(TEST_OWNER+4);
    assert(header[GRKIT_HOST_FRAME_OWNER/4]==TEST_OWNER);
    framing_leave(TEST_OWNER);
    assert(header[GRKIT_HOST_FRAME_OWNER/4]==0);
    assert(header[GRKIT_HOST_FRAME_CANVAS/4]==0);
    assert(header[GRKIT_HOST_FRAME_RATIO/4]==1);
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==0);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();

    /* The load cue starts at the first still live view and redraws it. */
    reset_cue(1000);
    framing_bind(TEST_OWNER);
    assert(header[GRKIT_HOST_LOAD_CUE/4]==1002);
    assert(invalidations==1 && invalidated==TEST_CANVAS);
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==1);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_cue();
    assert(header[GRKIT_HOST_FRAME_DRAWS/4]==1);
    assert(header[GRKIT_HOST_FRAME_BITMAP/4]==TEST_BITMAP);
    /* It stays for CUE_SECONDS without more redraws, then ends with one. */
    local_second=1000+CUE_SECONDS-1;
    framing_bind(TEST_OWNER);
    assert(header[GRKIT_HOST_LOAD_CUE/4]==1002 && invalidations==1);
    local_second=1000+CUE_SECONDS;
    framing_bind(TEST_OWNER);
    assert(header[GRKIT_HOST_LOAD_CUE/4]==0 && invalidations==2);
    /* The next draw clears the native bitmap once, as for Off. */
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==1);
    for(u32 i=1; i<=TEST_STRIDE*480; ++i) pixels[i]=0;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(header[GRKIT_HOST_FRAME_BITMAP/4]==0);
    for(u32 i=1; i<=TEST_STRIDE*480; ++i) assert(pixels[i]==0);
    framing_bind(TEST_OWNER);
    assert(invalidations==2);

    /* A cue that starts before midnight ends after it. */
    reset_cue(24*3600-2);
    framing_bind(TEST_OWNER);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    local_second=CUE_SECONDS-3;
    framing_bind(TEST_OWNER);
    assert(header[GRKIT_HOST_LOAD_CUE/4]==24*3600 && invalidations==1);
    local_second=CUE_SECONDS-2;
    framing_bind(TEST_OWNER);
    assert(header[GRKIT_HOST_LOAD_CUE/4]==0 && invalidations==2);

    /* The movie live view keeps the cue pending. */
    reset_cue(1000);
    BYTE(TEST_OWNER,LIVEVIEW_MOVIE)=1;
    framing_bind(TEST_OWNER);
    assert(header[GRKIT_HOST_LOAD_CUE/4]==CUE_PENDING && invalidations==0);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    BYTE(TEST_OWNER,LIVEVIEW_MOVIE)=0;
    framing_bind(TEST_OWNER);
    assert(header[GRKIT_HOST_LOAD_CUE/4]==1002 && invalidations==1);

    /* DISP Off hides the cue, and its time still runs. */
    reset_cue(1000);
    display_mode=DISPLAY_OFF;
    framing_bind(TEST_OWNER);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    /* Each other display shows it. */
    for(u32 mode=DISPLAY_FULL; mode<DISPLAY_OFF; ++mode) {
        reset_cue(1000);
        display_mode=mode;
        framing_bind(TEST_OWNER);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert_cue();
    }

    /* The cue shows in a native aspect where the layers do not. */
    reset_cue(1000);
    header[GRKIT_HOST_FRAME_RATIO/4]=LAST_RATIO;
    native_aspect=3;
    framing_bind(TEST_OWNER);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_cue();

    /* The cue does not start without the canvas of a still live view. */
    reset_cue(1000);
    WORD(TEST_OWNER,LIVEVIEW_CANVAS)=0;
    framing_bind(TEST_OWNER);
    assert(header[GRKIT_HOST_LOAD_CUE/4]==CUE_PENDING && invalidations==0);

    /* Without the profile, no point reads the counter. */
    assert(profile_clock==0);
    /* Hidden Guides clear their old pixels once, then stop requesting full
       clears. Both DISP policies keep the same visible guide pixels. */
    for(u32 policy=0;policy<2;++policy) for(u32 mode=0;mode<4;++mode) {
        reset(); register_conditional_guides(); GUIDE_STYLE=1; GUIDE_DISP=policy;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==1);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert(pixels[1+240]==0x80000000U);
        display_mode=mode;
        for(u32 i=1;i<=TEST_STRIDE*480;++i) pixels[i]=SENTINEL;
        BYTE(TEST_CANVAS,0xa0)=0;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==1);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        u32 visible=policy || mode<2;
        assert(pixels[1+240]==(visible ? 0x80000000U : SENTINEL));
        for(u32 draw=0;draw<3;++draw) {
            BYTE(TEST_CANVAS,0xa0)=0;
            framing_prepare(TEST_CANVAS);
            assert(BYTE(TEST_CANVAS,0xa0)==visible);
            framing_paint(TEST_CANVAS,TEST_BITMAP);
        }
        /* Returning to full information restores the guides. */
        display_mode=DISPLAY_FULL;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==1);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert(pixels[1+240]==0x80000000U);
        GUIDE_STYLE=0;
        framing_prepare(TEST_CANVAS); framing_paint(TEST_CANVAS,TEST_BITMAP);
        BYTE(TEST_CANVAS,0xa0)=0; framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==0);
    }
    /* ADJ ignores display flags but honors enabled conditions. */
    for(u32 policy=0;policy<2;++policy) {
        reset(); register_conditional_guides(); GUIDE_STYLE=0; GUIDE_DISP=policy;
        display_mode=DISPLAY_OFF;
        header[GRKIT_HOST_FRAME_PREVIEW/4]=1;
        header[GRKIT_HOST_MODULE_PREVIEW/4]=1;
        header[GRKIT_HOST_MODULE_PREVIEW/4+1]=1;
        header[GRKIT_HOST_FRAME_PREVIEW_ADJ/4]=TEST_OWNER;
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert(pixels[1+240]==0x80000000U);
    }
    /* A missing optional condition is false even with not_equals. */
    reset(); register_conditional_guides(); GUIDE_STYLE=1;
    WORD(TEST_LAYERS+28,0)=NO_SETTING; WORD(TEST_LAYERS+28,4)=2;
    assert(!layers_active(header));
    /* Framing already reads as Off with incompatible native geometry.
       After clearing an old mask once, later draws need no full clear. */
    reset(); GUIDE_STYLE=0;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    native_aspect=3;
    framing_prepare(TEST_CANVAS); framing_paint(TEST_CANVAS,TEST_BITMAP);
    BYTE(TEST_CANVAS,0xa0)=0; framing_prepare(TEST_CANVAS);
    assert(!layers_active(header) && BYTE(TEST_CANVAS,0xa0)==0);

    /* The development profile counts each point, and each module layer in
       the slot of the module whose code holds its paint function: Guides is
       module 0, Framing (the mask and the label) module 1. */
    reset();
    header[GRKIT_HOST_FRAME_RATIO/4]=1;
    GUIDE_STYLE=1;
    profile_memory[0]=0x5e000000; profile_memory[1]=0x5e000010;
    profile_memory[4]=0x5e000010; profile_memory[5]=0x5e000030;
    header[GRKIT_HOST_PROFILE/4]=TEST_PROFILE;
    framing_bind(TEST_OWNER);
    framing_prepare(TEST_CANVAS);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    {
        const uint32_t *slots=profile_memory+GRKIT_PROFILE_SLOTS/4;
#define SLOT(n,w) slots[(n)*GRKIT_PROFILE_SLOT_SIZE/4+(w)]
#define MODULE_SLOT(m,kind) (GRKIT_PROFILE_MODULE_BASE+(m)*GRKIT_PROFILE_KINDS+(kind))
        assert(SLOT(GRKIT_PROFILE_FRAME_BIND,0)==1 && SLOT(GRKIT_PROFILE_FRAME_BIND,1)==100);
        assert(SLOT(GRKIT_PROFILE_FRAME_PREPARE,0)==1 && SLOT(GRKIT_PROFILE_FRAME_PREPARE,2)==100);
        /* The paint holds its three layers: 100 cycles each, and 100 of its own. */
        assert(SLOT(GRKIT_PROFILE_FRAME_PAINT,0)==1 && SLOT(GRKIT_PROFILE_FRAME_PAINT,1)==700);
        assert(SLOT(MODULE_SLOT(0,GRKIT_PROFILE_PAINT),0)==1 && SLOT(MODULE_SLOT(0,GRKIT_PROFILE_PAINT),1)==100);
        assert(SLOT(MODULE_SLOT(1,GRKIT_PROFILE_PAINT),0)==2 && SLOT(MODULE_SLOT(1,GRKIT_PROFILE_PAINT),1)==200);
        assert(SLOT(MODULE_SLOT(1,GRKIT_PROFILE_PAINT),2)==100);
        /* A call of another core counter, and code outside each module, count nothing. */
        profile_add(TEST_PROFILE,GRKIT_PROFILE_FRAME_BIND,PROFILE_DISCARD);
        assert(SLOT(GRKIT_PROFILE_FRAME_BIND,0)==1);
        {
            u32 start=profile_begin(header);
            profile_core_id=1;
            profile_end(header,start,GRKIT_PROFILE_FRAME_BIND);
            profile_core_id=0;
            assert(SLOT(GRKIT_PROFILE_FRAME_BIND,0)==1);
        }
        profile_memory[1]=0x5e000000;
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert(SLOT(MODULE_SLOT(0,GRKIT_PROFILE_PAINT),0)==1);
        assert(SLOT(MODULE_SLOT(1,GRKIT_PROFILE_PAINT),0)==4);
        assert(SLOT(GRKIT_PROFILE_FRAME_PAINT,0)==2);
#undef SLOT
#undef MODULE_SLOT
    }
    return 0;
}
