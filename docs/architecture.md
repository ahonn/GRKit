# Architecture

## Domains and crates

GRKit has three execution domains. Rust tools run on the developer computer. The installer runs in camera Linux.
Menu, processing, and Framing hooks run in the camera's RTOS domain.

- `grkit-core` implements artwork, GRF formats, effect compilation, runtime templates, and deterministic ZIP assembly. It does not read files or start processes.
- `grkit-build` checks firmware, generates patches, and invokes Clang and LLD.
- `grkit-cli` exposes commands and enforces filesystem rules.

Native sources live under `runtime/`, and package documents live under `resources/`. The CLI embeds both, so it works outside the checkout.
Rust tests pin GRF bytes, artwork, generated headers, and loader scripts. Host C tests check the GRF parser, the effect catalog, the menu, and Framing.

## Loading

The loader script starts at power-on and polls. `lcmd` requests return NG until Linux mounts the card, so the loader repeats SCAN until the allocation plan appears.
SCAN selects the effect catalog and writes the plan. The startup script allocates the arrays and stays resident, so they remain allocated.

INSTALL checks the allocations, prepares records and icons, then publishes code before entry hooks.
It returns not-ready statuses while the camera initializes, and the loader repeats it. The result is `INSTALL.OK`, or `INSTALL.ERR` with the decimal status.
`O_EXCL` files serialize overlapping attempts. The loader deletes stale result and lock files at startup.

An unusable effect file does not stop the other effects or Framing. Unusable files include names outside the `NAME.GRF` 8.3 rule, symbolic links, unreadable files, and invalid records.
The `._` companions that macOS writes are therefore ignored.

With more than 20 effects, the installer keeps the 20 smallest effect IDs.
For a duplicate ID, the file with the smallest name wins. The result does not depend on directory order. Directory read errors stop the installation.

The 20-effect capacity is a product limit, not a GRF format limit. It fits one script array and one icon chunk.
The script array holds the header, the records, the 141,376-byte soft-glow map (376 x 376 cells), and the 21,620-byte live-view glow overlay.
Off, GRKit, and the fourteen module tiles (four Framing, five Guides, five Date Stamp) use sixteen host icon records outside that capacity.
The module tiles are contiguous, and the menu derives each one from the first Framing tile.

### Saved selections

After `INSTALL.OK`, the resident loader restores and saves the effect, the Framing ratio, the Guides style, and the Date Stamp style.
The private header is the loader's `registry` array, so the script reads and writes it directly.
It keeps one decimal integer in `SCRIPT/PLUGINS/STATE.TXT`: `code + 65536 * (ratio + 4 * (guide + 8 * stamp))`, where `code` is the effect's ImageTone code and 0 is Off.

The loader checks the state every 2 s and writes the file only after a change. It writes through the camera's own file system, as photos do, never from Linux while the camera may write photos.
An unreadable or out-of-range file restores nothing. An effect that is no longer installed stays Off, and its file entry stays until the next change.
A power loss during the write can lose the last change.

## Menu

The ADJ toolbar shows five native entries and a private GRKit entry. All six cells use a 50-pixel pitch with a 40-pixel icon.
A private LineView draws the native translucent background before the shared selection frame. The private icon draws after the frame.

The ADJ controller owns both objects. Exit hides them and restores the native geometry. Destruction unlinks and releases them. Movie mode keeps the five-entry layout.

The compiled registry in `runtime/modules.h` contains Image Control, Framing, Guides, and Date Stamp.
Each descriptor owns its title, summary, rows, labels, icons, selection checks, activation, and committed row. A module page opens at its committed row.
Module state stays in the private host header.

MENU returns one level. From the GRKit root it returns to the native ADJ items. The private menu restores the native list on its supported exit paths.
List restoration suppresses the native selection callbacks.

## Image Control

Selection changes a private active-record pointer. Each processing task captures that pointer once. Off follows the native processing path.
Records apply the native Standard adjustments, which the parameter hook copies from the live settings without writing them back.

A preview word overrides the active record in the live-view context only. Still captures always use the committed record.
Highlighting a row previews it. OK commits it. MENU, other exits, and controller destruction clear the preview.

The live-view diff at `0x536dc860` sets the stage flags (`+4`..`+0x12` of the tuning object) from native inputs only.
When the live-view record changes, the gamma hook therefore rebuilds the curves and sets the color matrix flag (`+5`).
The matrix stage runs before the gamma hook, so the matrix changes one update later.
Without this, the color matrix keeps the previous effect until the white balance changes. The matrix carries the monochrome weights.

Native Image Control pages apply each highlighted value, which previews native processing while the private effect stays committed.
OK or an ADJ press on the native slot, and OK on the native MENU page, disable the private effect.
MENU on the ADJ slot keeps it, because MENU emits its switch event before the same confirm event as OK.

Fn on an effect row commits that row and opens the native Standard parameter page. Off has no Fn hint and ignores Fn.
Fn uses the native slot's call: event `0x1b` with argument 2, then `ChangeController(adj, 0x2000014, 0x2000000)`.
It first refreshes the model snapshot with `0x531a0150`, because a cancel restores the snapshot type persistently. It requires the native Standard type.
On return, header `+252` reopens GRKit Image Control at the committed row.

Photos record the effect in the maker-note ImageTone tag (`0x4f`, hook `0x53608080`). Each record has a stable code at `+0x6f8`: `0x8000` plus 15 bits of an FNV-1a hash of the effect ID.
Native readers accept only 256 to 267, so in-camera RAW development falls back to Standard.

Playback extraction keeps a non-native tag in metadata key `0x25` (instruction `0x535c251c`). The DISP icon hook (`0x53179b7c`) shows the effect icon.
The detail hook (`0x532b7cfc`) shows GRKit photos as Standard. The still-viewfinder icon adapter substitutes the active effect icon outside movie mode.

## Framing

### Viewfinder

Framing draws black at 50% alpha into the still-viewfinder OSD bitmap. It draws after the canvas background clear and before the child views, so native status stays on top.
It checks the still mode, the native 3:2 aspect, the canvas owner, the dimensions, the pixel format, and the allocation first.
The OSD pixel buffer moves between boots (`0x5f83df80` and `0x619e1200` were seen), so the allocation must lie in `0x40000000`..`0x80000000`.

The ratio badge occupies `[204,46,66,22]`. A new session starts with the saved selections.
A still live view redraws its canvas only on a change. A selection restored at power-on has not drawn on the canvas yet. Such a selection invalidates the canvas (`0x533ad308`) on the next live-view update.

ADJ has its own canvas in slot zero (`controller+4`), so the live-view mask is not visible there.
While ADJ is open, Framing draws the committed ratio's mask on the ADJ canvas below the menu controls. The Framing page draws the highlighted ratio instead.

### Guides

Guides draws 1-pixel lines inside the photo area of the committed Framing ratio, in the same drawing path as the mask. Off uses the full 3:2 canvas.
The styles are thirds, a center cross with 20-pixel arms, the golden section (0.381966), and corner-to-corner diagonals.
Grid positions are fractions of the photo size in 1/65536, rounded. The diagonals are Bresenham lines, rounded half up. Neither needs division.

The line color is opaque gray (`0xffc0c0c0`), which has the same value with straight or premultiplied alpha. The format of the OSD bitmap is not confirmed.
The mask draws first and the badge last, so the badge covers the lines that cross it. Native child views, which may include the native grid guide, draw after both.

The Guides page previews its highlighted style on the ADJ canvas, inside the committed ratio. The Framing page previews its ratio with the committed style.
The preview style is in header `+48`, and it applies only while the Framing preview word is set.

### Capture

Capture uses the hidden GR IV aspect ids. Id 1 is 4:3 (5328x4000). Id 2 is 16:9 (6000x3376) or 65:24 (6000x2216). The menu, icon, and badge name 65:24 XPan.
The native StillAspectRatio value stays 0 (3:2), so menus, AF areas, and the live view stay native.
A stored 1 or 2 would also make the stock firmware assert in AF code after a boot without GRKit.

The still aspect property handler (`0x535423f4`, property `0xd4`) maps a 3:2 request to the Framing id. Other native values pass unchanged.
A remote or shell request for 3:2 also records the Framing ratio.
A Framing menu change calls the native setter with the unchanged 3:2 value. Each live-view update does the same when the property's next copy differs from the Framing id.

GR III implements only live-view modes 0 (3:2) and 3 (1:1). Four in-place changes keep ids 1 and 2 on the 3:2 paths:

- The live-view mode table at `0x53d553c8` maps ids 1 and 2 to mode 0.
- The mode lookup at `0x534371d8` reads its table only for id 3. Without this change the live view times out after a capture.
- The pipeline row function at `0x536a3200` returns row 0 for ids 1 and 2 without an assertion.
- The AF and touch rectangle function at `0x535ee890` uses the 3:2 rectangle for ids 1 and 2.

65:24 shares id 2. Hooks in GetResolution (`0x536a2a30`) select the id 2 heights and the screennail and thumbnail insets.
Shooting follows the Framing ratio. In playback, the size classifier (`0x532b3988`) records whether the shown photo has a 65:24 size.
The image aspect getter (`0x532b3ca0`) reports id 2 for 65:24 sizes.

65:24 sizes are 6000x2216, 4800x1776, 3360x1240, and 1920x712. Their heights are multiples of 8. The maker note records 16:9 for them.

DNG files keep the full 6112x4064 sensor data, as native 1:1 does. DefaultCropOrigin and DefaultCropSize describe the crop.
In-camera Crop, Resize, Digital Filter, and RAW development do not know the 65:24 variant.

## Effect extension

Bytes 84..127 of a GRFX0001 file hold an optional extension.
It holds u32 features at 84, an i16[9] signed Q13 post matrix at 88, and glow at 107. Byte 106 is reserved and zero.
Each feature bit owns its field, and a field without its bit stays zero. A monochrome effect cannot have a post matrix. An effect without features has zeros in the whole extension.

The installer copies the extension to record `+0x700`. Records are 0x740 bytes.

The processing region has little room left, so the extension hooks link into the Framing region (`runtime/picture.*`). The live-view diff hook moved there too.

### Post matrix

The YCbCrMatrix stage computes the hue rotation times BT.601 (`0x536e4720`), and its Make calls that at `0x536e4774`.
The hook multiplies the result by the effect's post matrix P: `CSP' = CSP x P`. P therefore acts on gamma-encoded RGB after the tone curves, and the Hue setting still rotates after it.

The coefficients are 9-bit two's complement with 256 = 1.0. The register writer packs 9 bits without saturation, so the hook sign-extends each one and saturates the products to -256..255.
The live-view diff also sets the YCbCr stage flag (`+7`) when the live-view record changes. Its stage runs before the gamma hook, so the post matrix changes one update later.

## Recorded pictures

`JpegEncodeMacroProcess::Execute` (`0x535047d4`) receives four image descriptors at configuration `+0x3c`.
They are the main picture, the 720 x 480 screennail, the 160 x 120 thumbnail, and a second full-size encode of the main planes.
Each descriptor holds Y at `+4`, CbCr at `+8`, and u16 width, height, and shared pitch at `+0x10`. The CbCr plane interleaves Cb and Cr for each pixel pair, in full rows (4:2:2) or half rows (4:2:0).
The entry hook processes each plane set once, before the encode is submitted. It cleans and invalidates the data cache for the planes before and after (`0x5383c8a8`), because the image hardware writes and reads them.

The screennail and the thumbnail have black bars that depend on the capture aspect. The hook uses the visible area from the GetResolution inset tables and the Framing insets.

### Soft glow

An effect's glow lifts the halo of highlights toward white and moves its chroma toward neutral, like a mist filter.
The hook samples Y above 200 into a map of at most 376 cells per side. Three box passes per axis spread it, and bilinear weights blend it back.
Cells without glow around them are skipped.

Glow uses the record of the last still build, so it applies to the picture that the build developed. It does not reach RAW files. The viewfinder shows an approximation (see [Live-view glow](#live-view-glow)).
Its time on a camera is not measured. Bright scenes take longest.

### Live-view glow

The live-view pipe has no spatial filter that can spread light. The viewfinder therefore draws an approximation on the still OSD canvas, below the Framing mask and the guides.

The overlay reads the frame that the LCD main layer shows. The input size is at `0x28904208`, the Y plane at `0x28904210`, and its pitch at `0x28904240`.
The scaler ratios are at `0x28904404` and `0x28904408`.
The live view keeps its frames in the 1 GiB pool at `0xa0000000`. The RTOS maps that pool one to one as normal cacheable memory, so the overlay cleans and invalidates the plane before it reads it.

The overlay samples a 180 x 120 map, one cell for each 4 x 4 LCD pixels. It uses the threshold, blur radius, and gain of the photo glow.
It writes premultiplied white. The native OSD stores premultiplied pixels: no translucent native pixel has a color above its alpha.
White at alpha a shows Y + (255 - Y) x a, the same lift as the photo. Chroma moves toward neutral by a instead of a / 2.

The still LCD controller updates about 200 times a second. The overlay therefore redraws on the generic timer (CNTPCT and CNTFRQ), eight times a second.
It redraws while it shows a halo or the frame has a highlight, and it checks the frame once per period. After the glow ends, one more redraw erases the overlay.

The UI task runs in SVC mode. In User mode the overlay stays off, because the timer and the cache maintenance need PL1.
The overlay has its own map after the photo glow map, because a capture can run while the viewfinder redraws.

The brightness, refresh, and cost of the overlay on a camera are not measured.

### Date stamp

The Date Stamp module prints the local time (`0x536b6510`) at the bottom right of the visible area, as a film data back exposes it.
The styles are `'YY M D`, `'YY M D  H:MM`, `M D 'YY`, and `D M 'YY`. Months, days, and hours have no leading zero.

The digits are italic seven-segment characters at 4.5% of the short side, at least 8 pixels. They render into a coverage mask of at most 48 rows (32 for long text), with horizontal anti-aliasing.
Two box blurs of the mask give an amber core and a red-orange halo. The hook scales the mask up and adds the light with a screen blend in RGB.
The digits therefore glow on dark areas and fade on bright ones. A monochrome GRKit effect gets the luma of the light only.

Only the largest plane of an encode renders the mask. The screennail and the thumbnail sample that mask at their own scale, with 2 x 2 subsamples for each pixel.
Playback shows the screennail first and then the main picture. The stamp therefore keeps its position and total width, and only the strokes become sharper.
If the largest plane has digits of 24 pixels or less, its mask has twice that size, and each pixel averages 2 x 2 cells.

The mask uses the glow scratch after the glow. The blur line copy is on the stack (1280 bytes), because the code region is read-only.
Every encode that passes through the hook gets the stamp, including in-camera RAW development and edits. Those show the time of the edit.

## Code areas

| Component | Range | Notes |
| --- | --- | --- |
| Processing | `0x53002080`–`0x53002340` | A32 assembly |
| Menu | `0x53002340`–`0x53004000` | Thumb-2 C at `-Oz`, A32 entry points |
| Framing | `0x540b6000`–`0x540be000` | A32 C at `-Os` and assembly: Framing, Guides, and the effect extension and picture hooks |

The translation table starts at `0x53004000` and must remain untouched. The freestanding link has no division helpers, so menu C must not divide.
Picture code divides with its own shift-subtract routine, once per picture for each reciprocal.

The Framing range follows the decoded RTOS image, which ends at `0x540b55c0`. The RTOS maps it read-only and executable up to `0x55000000`, and no firmware code refers to it.
BOOT loads only the image, so the range can keep the previous session's code after a restart. The installer therefore overwrites it without a stock-byte check.
All other patches keep that check. The builder rejects overflow, occupied reservations, changed hook instructions, and overlapping patches.

Cache synchronization and camera task lifetimes need device testing whenever their implementation changes.
