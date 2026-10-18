# Architecture

## Domains and crates

GRKit has three execution domains. Rust tools run on the developer computer. The installer runs in camera Linux.
Menu, processing, and Framing hooks run in the camera's RTOS domain.

- `grkit-core` implements artwork, GRF formats, effect compilation, runtime templates, and deterministic ZIP assembly. It does not read files or start processes.
- `grkit-build` checks firmware, generates patches, and invokes Clang and LLD.
- `grkit-cli` exposes commands and enforces filesystem rules.
- `grkit-wasm` assembles packages in the browser. It takes a runtime build as `grkit build` writes it (`build.json` and `sd/`) and compiled effects, and it uses the `grkit-core` checks that the CLI uses.
- `web/` is the static site. It fetches the runtime, the effect catalog, and the selected effects, and calls `grkit-wasm` to build `GRKit.zip` in the browser.

The site preview applies one 33 x 33 x 33 table for each effect. `grkit effects publish` computes each table from the fit model: it undoes the Standard rendering of an sRGB input, then renders the result with the effect.
An effect equal to Standard maps each input to itself. The preview assumes that the input is a Standard JPEG. The long highlight shoulder of the base gamma can exaggerate the change of other bright, saturated colors.

## Cameras

GR IIIx 1.60 runs the RTOS code of GR III 2.10. The two RTOS images have the same size and layout. They differ only in the model name, the version, the build time, the default year, the ModeSet numbers, and one byte of the model selection.
None of these bytes is a patch site or a firmware input of GRKit. A runtime built from either firmware is therefore byte-identical, and both cameras run the `gr3-2.10` runtime.

The cameras differ in the ModeSet file that enables SD scripts: `00078350.588` on GR III and `00078490.609` on GR IIIx. `grkit_core::camera` lists the cameras.
A package holds the ModeSet file of one camera, and its installation guide names that camera and file. `grkit verify` identifies the camera from that file.

GR IV 1.11 is a different build, with the target `gr4-1.11`. GR IV and GR IV HDF run the same firmware and ModeSet file, `00078560.636`.
Its RTOS image is larger (0x13d2ac0 bytes), and its data, functions, and object layouts are at other addresses. `runtime/targets/<target>.h` holds these values for the C and assembly sources.

The build defines `GRKIT_TARGET_<target>` (`components::Layout::define`), and `runtime/firmware.h` selects the header from it. Without a known definition, the compilation stops.
`components::Layout` holds the regions and hooks of each target, and the build generates `hooks.h` from it.

The GR IV runtime has the menu, Image Control, Framing, Guides, and Date Stamp. GR IV Framing offers only XPan, as a crop of native 16:9 (see Framing).
GR IV processing hooks are in `runtime/targets/gr4-1.11-processing.S`, after the Framing region. GR IV reads the type tables through getters.
A private record therefore sets the Standard index (11) where all type paths join, and the record store keeps the private record.

The GR IV packet tail has no special type path. Its gamma object holds the built-curve flag at `+0x382c`. `target::ImageControl` holds the tables, the Standard record, and the settings offsets of each target.

GR IV photos record the plugin effect as on GR III. The hooks are the ImageTone call `0x53810804`, the DISP icon `0x531916c0`, and the detail `0x533a7614`.
GR IV extraction compares each native tag value, and its default case (`0x537d8a54`) keeps a non-native tag. GR IV also uses the native values `0x8100` to `0x8105`, so the installer skips codes `0x8100` to `0x81ff` there.
The effects use the GR III fit model.

Neither camera reads one fixed base gamma table. Each picture build computes the base from 15 ROM curves, the tone setting, and a gain. GR IV uses the GR III function (`0x5391dad4`) and the same curves.
`grkit effects publish` reads the curve for tone 0 and the lowest gain (GR IV `0x5407f4b4`). The gain of real GR IV stills is not known, so their shadows can differ from the fit.
The GR IV Standard look matrix differs from the GR III one.

GR IV icon descriptors have 12 bytes, and GR IV draws icons from a copy of the icon pixels. The installer reads the base of that copy from the icon drawer.

Native sources live under `runtime/`, and package documents live under `resources/`. The CLI embeds both, so it works outside the checkout.
Rust tests pin GRF bytes, artwork, generated headers, and loader scripts. Host C tests check the GRF parser, the effect catalog, the menu, and Framing on GR III, and the picture code on both targets. A syntax check compiles the C sources of each target.

## Loading

The loader script starts at power-on and polls. `lcmd` requests return NG until Linux mounts the card, so the loader repeats SCAN until the allocation plan appears.

The loader waits 3 s before its first `lcmd`. The first request creates the firmware's host-interface command factory.
In some starts, that creation took a data abort in the Script task during the first second.
Linux mounts the card about 5 s after power-on, so the wait does not delay the installation.

SCAN selects the effect catalog and writes the plan. The startup script allocates the arrays and stays resident, so they remain allocated.

INSTALL checks the allocations, prepares records and icons, then publishes code before entry hooks.
It returns not-ready statuses while the camera initializes, and the loader repeats it. The result is `INSTALL.OK`, or `INSTALL.ERR` with the decimal status.
`O_EXCL` files serialize overlapping attempts. The loader deletes stale result and lock files at startup.

An unusable effect file does not stop the other effects or Framing. Unusable files include names outside the `NAME.GRF` 8.3 rule, symbolic links, unreadable files, and invalid records.
The `._` companions that macOS writes are therefore ignored.

INSTALL also reads `SCRIPT/PLUGINS/MODULES.TXT`, which names the modules that the menu hides (`runtime/visibility.h`). A missing file hides nothing.
An unknown or repeated name stops the installation with status 43, and a read error with status 44. The installer stores the result in header `+56`.
Hidden modules keep their hooks. Each one stays inactive at Off.

With more than 20 effects, the installer keeps the 20 smallest effect IDs.
For a duplicate ID, the file with the smallest name wins. The result does not depend on directory order. Directory read errors stop the installation.

The 20-effect capacity is a product limit, not a GRF format limit. It fits one script array and one icon chunk.
The script array holds the header, the records, and the 141,376-byte picture scratch for the date stamp masks.
Off, GRKit, and the fourteen module tiles (four Framing, five Guides, five Date Stamp) use sixteen host icon records outside that capacity.
The module tiles are contiguous, and the menu derives each one from the first Framing tile.

### Saved selections

After `INSTALL.OK`, the resident loader restores and saves the effect, the Framing ratio, the Guides style, and the Date Stamp style.
The private header is the loader's `registry` array, so the script reads and writes it directly.
It keeps one decimal integer in `SCRIPT/PLUGINS/STATE.TXT`: `code + 65536 * (ratio + 4 * (guide + 8 * stamp))`, where `code` is the effect's ImageTone code and 0 is Off.

The loader checks the state every 2 s and writes the file only after a change. It writes through the camera's own file system, as photos do, never from Linux while the camera may write photos.
An unreadable or out-of-range file restores nothing. An effect that is no longer installed stays Off, and its file entry stays until the next change.

A hidden module restores Off, because no menu can turn it off. The script has no bit operators, so it reads header `+56` by division.
A power loss during the write can lose the last change.

## Menu

The ADJ toolbar shows five native entries and a private GRKit entry. All six cells use a 50-pixel pitch with a 40-pixel icon.
A private LineView draws the native translucent background before the shared selection frame. The private icon draws after the frame.

The ADJ controller owns both objects. Exit hides them and restores the native geometry. Destruction unlinks and releases them. Movie mode keeps the five-entry layout.

The compiled registry in `runtime/modules.h` contains Image Control, Framing, Guides, and Date Stamp.
Each descriptor owns its title, summary, rows, labels, icons, selection checks, activation, and committed row. A module page opens at its committed row.

The root lists only the visible modules, in registry order. Image Control is visible when effects are installed. Each other module is visible unless header `+56` has its bit.
If no module is visible, Image Control shows with only Off. If one module is visible, ADJ opens its page directly, and MENU or left returns to native ADJ.
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

GR IV has native 4:3 and 16:9 live-view modes, so the GR III changes above would break them. GR IV Framing therefore offers Off and XPan, and XPan crops native 16:9 photos.
GR IV has no capture request hook and no live-view changes. The GR IV sizes come from `ImageResolutionTranslatorKb636`. Its GetResolution (`0x538cfbfc`) has the id 2 height hooks, and slots 9 and 10 have the inset hooks.
The `Kb588` class holds the GR III sizes and is not used. The GR IV size classifier (`0x533a2820`) keeps the width in `r6` and the class pointer in `r5`.

GR IV 65:24 sizes are 6192x2288, 4944x1824, 3504x1296, and 1920x712. In native 16:9 the XPan mask uses the GR III XPan area. In native 3:2 XPan has no effect, and the viewfinder shows only the guides.
The date stamp follows the same rule: it uses the XPan area of the screennail and the thumbnail only for a native 16:9 capture.

## Effect extension

Bytes 84..127 of a GRFX0001 file hold an optional extension.
It holds u32 features at 84, an i16[9] signed Q13 post matrix at 88 (bit 1), and selective color parameters at 106 (bit 2). Bytes 120..127 are reserved and zero.
Each feature bit owns its field, and a field without its bit stays zero. A monochrome effect has no features.

An effect without features has zeros in the whole extension. Bit 4 is reserved, and the parsers reject it.

The installer copies the extension to record `+0x700`: features at `+0x700`, the post matrix at `+0x704`, and selective color at `+0x716`. Records are 0x740 bytes.

The processing region has little room left, so the extension hooks and the live-view diff hook link into the Framing region (`runtime/picture.*`).

### Post matrix

The YCbCrMatrix stage computes the hue rotation times BT.601 (`0x536e4720`), and its Make calls that at `0x536e4774`.
The hook multiplies the result by the effect's post matrix P: `CSP' = CSP x P`. P therefore acts on gamma-encoded RGB after the tone curves, and the Hue setting still rotates after it.

The coefficients are 9-bit two's complement with 256 = 1.0. The register writer packs 9 bits without saturation, so the hook sign-extends each one and saturates the products to -256..255.
The live-view diff also sets the YCbCr stage flag (`+7`) when the live-view record changes. Its stage runs before the gamma hook, so the post matrix changes one update later.

### Selective color

Selective color keeps the chroma of one hue range in recorded pictures and removes it elsewhere. The hue is the angle of (Cb - 128, Cr - 128).
The parameters are an i16 Q12 direction (cos and sin of the target hue), then u16 Q8 tangents and a u8 chroma gate.
Each side of the target has an inner and an outer half width. Inside the inner one a pair keeps all of its chroma, and outside the outer one none.
Between them, the weight falls linearly in tan(offset).

The side toward larger angles is separate, because skin lies only on that side of red. The gate fades out pairs whose chroma along the target is low.

The effect compiler converts degrees to these integers, so the runtime needs no trigonometry. The parsers require a direction of length 0.95 to 1.03, inner < outer <= 1024, and c0 < c1 <= 181.

For each picture, `picture_encode` builds a 128 x 128 weight table in the picture scratch with the shift-subtract division. One cell covers a 2 x 2 block of Cb and Cr values.
Then each pair costs one lookup and two multiplies: 22 A32 instructions in the loop.
`grkit_core::selective` computes the same table, and a test compares the two byte for byte. Site previews use it.

The live view shows the effect in monochrome. `picture_post` reads the tuning context byte at `settings + 0x82ac` through the Make's third argument (`settings + 0x120`): 1 is ImageTuningLiveview and 0 a still build.
For a live-view build of a selective effect it clears the Cb and Cr rows of the YCbCr matrix. The viewfinder then shows the luma that the removed hues keep in the picture, and the target hue is gray too.
Still builds keep the whole matrix, and their encode applies selective color. The encode time on a camera is not measured.

## Recorded pictures

`JpegEncodeMacroProcess::Execute` (`0x535047d4`) receives four image descriptors at configuration `+0x3c`.
They are the main picture, the 720 x 480 screennail, the 160 x 120 thumbnail, and a second full-size encode of the main planes.
Each descriptor holds Y at `+4`, CbCr at `+8`, and u16 width, height, and shared pitch at `+0x10`. The CbCr plane interleaves Cb and Cr for each pixel pair, in full rows (4:2:2) or half rows (4:2:0).

The entry hook processes each plane set once, before the encode is submitted.
It applies selective color first and then the date stamp, so the stamp keeps its color. It cleans and invalidates the data cache for the planes before and after (`0x5383c8a8`), because the image hardware writes and reads them.

The screennail and the thumbnail have black bars that depend on the capture aspect. The hook uses the visible area from the GetResolution inset tables and the Framing insets.

### Date stamp

The Date Stamp module prints the local time (`0x536b6510`) at the bottom right of the visible area, as a film data back exposes it.
The styles are `'YY M D`, `'YY M D  H:MM`, `M D 'YY`, and `D M 'YY`. Months, days, and hours have no leading zero.

The digits are italic seven-segment characters at 4.5% of the short side, at least 8 pixels. They render into a coverage mask of at most 48 rows (32 for long text), with horizontal anti-aliasing.
Two box blurs of the mask give an amber core and a red-orange halo. The hook scales the mask up and adds the light with a screen blend in RGB.
The digits therefore glow on dark areas and fade on bright ones. A monochrome GRKit effect gets the luma of the light only.

Only the largest plane of an encode renders the mask. The screennail and the thumbnail sample that mask at their own scale, with 2 x 2 subsamples for each pixel.
Playback shows the screennail first and then the main picture. The stamp therefore keeps its position and total width, and only the strokes become sharper.
If the largest plane has digits of 24 pixels or less, its mask has twice that size, and each pixel averages 2 x 2 cells.

The masks use the picture scratch after the records. The blur line copy is on the stack (1280 bytes), because the code region is read-only.
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
