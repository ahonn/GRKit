#include <assert.h>
#include <stdint.h>
#include <string.h>

#define TEST_OWNER 0x1000U
#define TEST_CANVAS 0x2000U
#define TEST_BITMAP 0x3000U
#define TEST_PIXELS 0x60000000U
#define TEST_STRIDE 736U
#define SENTINEL 0x76543210U
static uint32_t owner_memory[256], canvas_memory[256], bitmap_memory[256];
static uint32_t pixels[1024 * 480 + 2];
static uint32_t native_aspect, capture_aspect, capture_requests;

static void *mapped(uint32_t address, uint32_t bytes)
{
    uint32_t *object;
    if (address >= TEST_OWNER && address < TEST_OWNER + sizeof(owner_memory)) {
        object = owner_memory; address -= TEST_OWNER;
    } else if (address >= TEST_CANVAS && address < TEST_CANVAS + sizeof(canvas_memory)) {
        object = canvas_memory; address -= TEST_CANVAS;
    } else {
        assert(address >= TEST_BITMAP && address < TEST_BITMAP + sizeof(bitmap_memory));
        object = bitmap_memory; address -= TEST_BITMAP;
    }
    assert(address + bytes <= sizeof(owner_memory));
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
#define FRAMING_CAPTURE_ASPECT() capture_aspect
/* The native setter resends 0 through the property handler hook. */
#define FRAMING_REQUEST_ASPECT() (++capture_requests, capture_aspect=framing_capture_request(0))
static uint32_t invalidations, invalidated;
#define FRAMING_INVALIDATE(view) (++invalidations, invalidated = (view))
#include "framing.c"

static volatile u32 header[64];
volatile u32 *framing_header(void) { return header; }

static void reset(void)
{
    memset((void *)header, 0, sizeof(header));
    memset(owner_memory, 0, sizeof(owner_memory));
    memset(canvas_memory, 0, sizeof(canvas_memory));
    memset(bitmap_memory, 0, sizeof(bitmap_memory));
    for (u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) pixels[i]=SENTINEL;
    WORD(TEST_OWNER, 0x15c)=TEST_CANVAS;
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
    header[GRKIT_HOST_ACTIVE/4]=0x59001000;
    header[GRKIT_HOST_FRAME_RATIO/4]=1;
    native_aspect=0;
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
    assert(header[GRKIT_HOST_FRAME_SKIPPED/4]==1);
}
/* These raster fixtures pin the badge independently of the runtime glyph table.
   Each character cell has five columns and a one-column gap, scaled by two. */
static const char *badge_rows[][7] = {
    {
        "#...#.####.............",
        "#...#.#...#............",
        ".#.#..#...#..###..#.##.",
        "..#...####......#.##..#",
        ".#.#..#......####.#...#",
        "#...#.#.....#...#.#...#",
        "#...#.#......####.#...#"
    },
    {
        "..#.....##.........###.",
        ".##....#.....##...#...#",
        "..#...#......##...#...#",
        "..#...####.........####",
        "..#...#...#..##.......#",
        "..#...#...#..##......#.",
        ".###...###.........##.."
    },
    {
        "...#.........###.",
        "..##...##...#...#",
        ".#.#...##.......#",
        "#..#..........##.",
        "#####..##.......#",
        "...#...##...#...#",
        "...#.........###."
    }
};
static u32 expected_badge(u32 ratio,u32 x,u32 y,u32 background)
{
    if(!ratio || x<204 || x>=270 || y<46 || y>=68) return background;
    if(x>=208 && y>=50 && y<64) {
        const char *line=badge_rows[ratio-1][(y-50)/2];
        u32 column=(x-208)/2;
        if(column<strlen(line) && line[column]=='#') return 0xffffffff;
    }
    return 0x80000000;
}
/* Independent guide fixtures: exact positions, rounded half up. */
static const u32 boxes[][4]={{0,0,720,480},{0,107,720,373},{0,37,720,442},{40,0,680,480}};
static const u32 thirds[][4]={{240,480,160,320},{240,480,196,284},{240,480,172,307},{253,467,160,320}};
static const u32 golden[][4]={{275,445,183,297},{275,445,209,271},{275,445,192,287},{284,436,183,297}};
static u32 expected_guide(u32 ratio,u32 style,u32 x,u32 y)
{
    const u32 *box=boxes[ratio];
    if(x<box[0] || x>=box[2] || y<box[1] || y>=box[3]) return 0;
    if(style==1 || style==3) {
        const u32 *lines=style==1 ? thirds[ratio] : golden[ratio];
        return x==lines[0] || x==lines[1] || y==lines[2] || y==lines[3];
    }
    /* Every center is (360,240), with 20-pixel arms. */
    if(style==2) return (y==240 && x>=340 && x<=380) || (x==360 && y>=220 && y<=260);
    if(style==4) {
        u32 width=box[2]-box[0], height=box[3]-box[1], column=x-box[0];
        u32 rows[2]={column, width-1-column};
        for(u32 i=0; i<2; ++i)
            if(y==box[1]+(2*rows[i]*(height-1)+width-1)/(2*(width-1))) return 1;
    }
    return 0;
}
static void assert_guides(u32 ratio,u32 style,u32 badge)
{
    const u32 *box=boxes[ratio];
    for(u32 y=0; y<480; ++y) for(u32 x=0; x<TEST_STRIDE; ++x) {
        u32 expected=SENTINEL;
        if(x<720) {
            if(ratio && (x<box[0] || x>=box[2] || y<box[1] || y>=box[3])) expected=0x80000000;
            if(expected_guide(ratio,style,x,y)) expected=GUIDE_COLOR;
            if(badge) expected=expected_badge(ratio,x,y,expected);
        }
        assert(pixels[1+y*TEST_STRIDE+x]==expected);
    }
}
int main(void)
{
    /* Fixed rectangles are independent expected output, not a second renderer. */
    const u32 rectangles[][4]={{0,0,720,480},{0,107,720,373},{0,37,720,442},{40,0,680,480}};
    for (u32 ratio=0; ratio<4; ++ratio) {
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
    /* Badge replacement erases old characters even before a native full clear. */
    reset();
    const u32 transitions[]={1,2,3,1,3,2};
    for(u32 i=0; i<sizeof(transitions)/sizeof(transitions[0]); ++i) {
        u32 ratio=transitions[i];
        header[GRKIT_HOST_FRAME_RATIO/4]=ratio;
        BYTE(TEST_CANVAS,0xa0)=0;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==1);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        for(u32 y=46; y<68; ++y) for(u32 x=204; x<270; ++x)
            assert(pixels[1+y*TEST_STRIDE+x]==expected_badge(ratio,x,y,SENTINEL));
        assert(header[GRKIT_HOST_FRAME_DRAWS/4]==i+1);
    }
    /* The ADJ preview canvas shows the highlighted ratio's mask without a badge. */
    reset();
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_FRAME_PREVIEW/4]=3;
    header[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=TEST_CANVAS;
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
    for(u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) pixels[i]=SENTINEL;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    assert(header[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]==TEST_CANVAS);
    /* An ended preview gets one more full draw, then releases the canvas. */
    header[GRKIT_HOST_FRAME_PREVIEW/4]=0;
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==1);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(header[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]==0);
    assert_untouched();
    /* Unsupported bitmaps stay untouched in the preview path too. */
    reset();
    header[GRKIT_HOST_FRAME_PREVIEW/4]=2;
    header[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=TEST_CANVAS;
    BYTE(TEST_BITMAP,2)=4;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    /* Guides stay inside the photo area of each ratio, below the badge. */
    for (u32 ratio=0; ratio<4; ++ratio) for (u32 style=1; style<=4; ++style) {
        reset();
        header[GRKIT_HOST_FRAME_RATIO/4]=ratio;
        header[GRKIT_HOST_GUIDE/4]=style;
        framing_prepare(TEST_CANVAS);
        assert(BYTE(TEST_CANVAS,0xa0)==1);
        framing_paint(TEST_CANVAS,TEST_BITMAP);
        assert_guides(ratio,style,1);
        assert(header[GRKIT_HOST_FRAME_DRAWS/4]==1);
        assert(header[GRKIT_HOST_FRAME_BITMAP/4]==TEST_BITMAP);
    }
    /* The diagonals run corner to corner. */
    for (u32 ratio=0; ratio<4; ++ratio) {
        const u32 *box=boxes[ratio];
        assert(expected_guide(ratio,4,box[0],box[1]) && expected_guide(ratio,4,box[2]-1,box[3]-1));
        assert(expected_guide(ratio,4,box[2]-1,box[1]) && expected_guide(ratio,4,box[0],box[3]-1));
    }
    /* Turning the guides off requests one more full clear. */
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_GUIDE/4]=0;
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==1);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert(header[GRKIT_HOST_FRAME_BITMAP/4]==0);
    BYTE(TEST_CANVAS,0xa0)=0;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==0);
    /* Unknown styles draw nothing. */
    reset();
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_GUIDE/4]=GUIDE_LAST+1;
    framing_prepare(TEST_CANVAS);
    assert(BYTE(TEST_CANVAS,0xa0)==0);
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    /* The ADJ preview draws guides without a badge, with or without a ratio. */
    reset();
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    header[GRKIT_HOST_FRAME_PREVIEW/4]=1;
    header[GRKIT_HOST_GUIDE_PREVIEW/4]=1;
    header[GRKIT_HOST_FRAME_PREVIEW_CANVAS/4]=TEST_CANVAS;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_guides(0,1,0);
    for(u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) pixels[i]=SENTINEL;
    header[GRKIT_HOST_FRAME_PREVIEW/4]=3;
    header[GRKIT_HOST_GUIDE_PREVIEW/4]=4;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_guides(2,4,0);
    assert(header[GRKIT_HOST_FRAME_DRAWS/4]==0);
    for(u32 i=0; i<sizeof(pixels)/sizeof(pixels[0]); ++i) pixels[i]=SENTINEL;
    header[GRKIT_HOST_GUIDE_PREVIEW/4]=GUIDE_LAST+1;
    framing_paint(TEST_CANVAS,TEST_BITMAP);
    assert_untouched();
    /* Native 3:2 requests record the Framing ratio; other native ratios pass. */
    {
        const u32 ids[]={0,2,2,1};
        for(u32 ratio=0; ratio<4; ++ratio) {
            header[GRKIT_HOST_FRAME_RATIO/4]=ratio;
            assert(framing_capture_request(0)==ids[ratio]);
            assert(framing_capture_request(3)==3);
            assert(framing_capture_request(1)==1);
        }
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
    header[GRKIT_HOST_GUIDE/4] = 2;
    framing_bind(TEST_OWNER);
    assert(invalidations == 2);
    /* Returning to the live view ends the playback 65:24 variant. */
    reset();
    ((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]=2;
    framing_bind(TEST_OWNER);
    assert(((volatile u8 *)header)[GRKIT_HOST_FRAME_FILE]==0);
    /* Live-view updates resend the 3:2 request only when capture differs. */
    reset();
    assert(capture_requests==1 && capture_aspect==2);
    framing_bind(TEST_OWNER);
    assert(capture_requests==1);
    header[GRKIT_HOST_FRAME_RATIO/4]=3;
    framing_bind(TEST_OWNER);
    assert(capture_requests==2 && capture_aspect==1);
    header[GRKIT_HOST_FRAME_RATIO/4]=0;
    framing_bind(TEST_OWNER);
    assert(capture_requests==3 && capture_aspect==0);
    header[GRKIT_HOST_FRAME_RATIO/4]=2;
    native_aspect=3; capture_aspect=3;
    framing_bind(TEST_OWNER);
    assert(capture_requests==3 && capture_aspect==3);
    native_aspect=0; capture_aspect=0;
    BYTE(TEST_OWNER,0x2d2)=1;
    framing_bind(TEST_OWNER);
    assert(capture_requests==3);
    BYTE(TEST_OWNER,0x2d2)=0;
    framing_bind(TEST_OWNER);
    assert(capture_requests==4 && capture_aspect==2);
    reset(); WORD(TEST_OWNER,4)=TEST_CANVAS+4; assert_rejected();
    reset(); BYTE(TEST_OWNER,0x2d2)=1; assert_rejected();
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
    reset(); native_aspect=3; assert_rejected();
    reset(); header[GRKIT_HOST_FRAME_RATIO/4]=4;
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
    return 0;
}
