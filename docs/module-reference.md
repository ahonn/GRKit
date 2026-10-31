# Module reference

This document describes the module interface of GRKit. The interface is experimental in GRKit 1.x: a minor release can change it, and its changelog then says so. [modules.md](modules.md) shows how to write, test, build, and install a module.

The sources of each contract:

| Contract | Source |
| --- | --- |
| C interface | `runtime/sdk/grkit.h` |
| Host test harness | `runtime/sdk/grkit_test.h` |
| Manifest and text | `crates/grkit-build/src/module.rs` |
| File format | `crates/grkit-core/src/grm.rs` |
| Installer checks | `runtime/module_link.h`, `runtime/install_plan.h`, `runtime/install_modules.h` |
| Capabilities | `crates/grkit-core/src/capabilities.rs` |

## Module folder

| Path | Required | Contents |
| --- | --- | --- |
| `module.toml` | yes | the manifest |
| `text/<language>.toml` | with a page | the page text in one menu language: `en`, `zh-Hans`, `zh-Hant`, `ja`, `ko` |
| C sources | no | the code that `sources` names. A module with only data has none. |
| `test.c` | with C sources | the host test of `grkit module test` |
| tiles | with a choice page | the 40 x 40 PNG files that `tiles` names |

The installer loads the modules of the package from `SCRIPT/GRKIT/MODULES` and the modules that a person adds from `GRKIT/MODULES`. A file in `GRKIT/MODULES` replaces the package module with the same name.

## module.toml

The manifest is TOML. An unknown key is an error.

### Top-level keys

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | string | required | The module ID: 1 to 8 characters `A`-`Z`, `0`-`9`, or `_`. The file name is `<id>.GRM`, and each setting name starts with it. |
| `version` | integer | required | The module version. The build writes it into the file header. GRKit does not read it. |
| `order` | integer | 100 | The default place of the module in ADJ and in the GRKit section of MENU, 0 to 65535: a smaller order first, then by ID. The official modules use 10 (Film Look) to 50 (Grain). ADJ Order in MENU changes the place in ADJ. |
| `sources` | array of strings | `[]` | The C source files, relative to the manifest. |
| `tiles` | array of strings | `[]` | The tile files, relative to the manifest. Each is a 40 x 40 PNG. At most 64. |
| `uses` | array of strings | `[]` | The settings of other modules or of the core that the module uses: `MODULE.KEY`, or `?MODULE.KEY` for an optional one. At most 8. See [Used settings](#used-settings). |
| `settings` | array of tables | `[]` | The settings of the module. At most 16. |
| `page` | table | none | The menu page. A module has at most one. |
| `layers` | array of tables | `[]` | The live-view layers. |
| `filters` | array of tables | `[]` | The plane filters. |

A manifest gives at least one page, layer, or filter. Each page, layer, and filter is one registration, and a module has at most 16 registrations.

A setting reference in `page`, `layers`, or `filters` is a key of `settings`, or a name of `uses` without its `?`.

### settings

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `key` | string | required | 1 to 15 characters `A`-`Z`, `0`-`9`, or `_`. Unique in the module. The full name is `<id>.<key>`. |
| `values` | integer | required | The number of values, 1 to 65536. The values are 0 to `values` - 1. |
| `saved` | boolean | `false` | The state file keeps the value. |
| `public` | boolean | `false` | Other modules can use the setting. |

### page

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `setting` | string | required | The setting of the page: a setting reference. |
| `off_row` | boolean | `false` | Row 0 shows the Off icon instead of tile 0. |
| `settings` | array of strings | `[]` | The rows of a native settings page: the page setting, then keys of `settings` or names of `uses`. Give 0 to 5 keys (`GRKIT_SETTING_ROWS`). The row of an optional used setting that no module has is left out. With one key, the page setting alone, the row gets a label of its own, and Fn opens no settings page. |
| `options` | boolean | `false` | The settings page of the module in the GRKit section of MENU has an Options row, which turns each value of the page on or off in ADJ, or each effect for a page of `CORE.TONE` (`GRKIT_CHOICE_OPTIONS`). A page of a setting with options needs `off_row` and at most 16 values. Without room on the settings page, its first rows leave it. |

A page of `CORE.TONE` has no `off_row` and no `settings`, and it needs no tiles. Its text has only `title`.

### layers

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `paint` | string | required | The C function `void paint(const Surface *s, u32 value)`. |
| `setting` | string | none | The setting whose value the layer paints: a setting reference. Without it, the value is always 1. |
| `order` | integer | required | The paint order, 0 to 65535. |
| `follow_disp` | boolean | `false` | The layer is off while DISP hides the native information. |
| `full_disp` | boolean | `false` | The layer is on only while DISP shows the full information. |
| `live_only` | boolean | `false` | The layer does not paint on the ADJ canvas. |

A layer has `follow_disp` or `full_disp`, not both.

### filters

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `apply` | string | required | The C function `void apply(PictureSet *set, u32 value)`. |
| `setting` | string | required | The setting whose value the filter applies: a setting reference. |
| `stage` | string | required | `frame` or `mark`. See [Plane filters](#plane-filters). |

### events

At most one table. It registers the event subscriber of the module.

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `handle` | string | required | The C function `void handle(u32 event, u32 a, u32 b)`. |
| `on` | array of strings | required | The events: `system_ready`, `setting_changed`. Each at most once. |

## Text files

A module with a page has one text file for each menu language: `text/en.toml`, `text/zh-Hans.toml`, `text/zh-Hant.toml`, `text/ja.toml`, and `text/ko.toml`. An unknown key is an error.

| Key | Type | Description |
| --- | --- | --- |
| `title` | string | The name of the page. The GRKit section of MENU shows the title of the first page as the module name. |
| `rows` | array of strings | One label for each value of the page setting. A page of `CORE.TONE` has none. |
| `summaries` | array of strings | Optional. The value in the root row, one for each value. Without it, the root row shows the label. |
| `settings.<KEY>.label` | string | The label of a row of the native settings page. |
| `settings.<KEY>.values` | array of strings | The text of each value of that row. The row of the page setting can omit it and use `rows`. |

The root row shows `<title><separator><summary>`. Each text, and the text of the root row, has at most 64 UTF-16 units. For a page of a used setting, the build takes the number of values from `rows` in `text/en.toml`, and the installer checks it against the setting.

## Tiles

A tile is a 40 x 40 PNG file. Tile n shows value n of the page setting, on its row and on the root row, and tile 0 is also the module tile. A choice page needs at least one tile for each value.

## Settings

The core stores the value of each setting, 0 to `values` - 1, and 0 until the user or a module changes it. A module reads and changes a value only through `setting_get` and `setting_set`. Read a setting each time that you use it, because the menu and other modules can change it.

### Handles

A function names a setting by its handle, a `u32`.

| Handle | Setting |
| --- | --- |
| `GRKIT_SETTING(n)` | setting n of this module |
| `GRKIT_USED(n)` | used setting n, in the order of `uses` |
| `SETTING_<KEY>` | `GRKIT_SETTING(n)` of the setting `<KEY>`, from the generated `grkit_module.h` |
| `USED_<MODULE>_<KEY>` | `GRKIT_USED(n)` of the used setting `<MODULE>.<KEY>`, from the generated `grkit_module.h` |
| `GRKIT_NO_SETTING` | an optional used setting that no loaded module has. `setting_get` returns 0. |

### Saved settings

The state file keeps a saved setting under a number that the core computes from its full name, `MODULE.KEY`:

- A new ID or a new key starts the setting at 0 again.
- If a new version has fewer values, a saved value out of the range is not restored.
- INSTALL restores a saved value only when its module loads. If the module is Off or missing, the file keeps the value.
- The resident loader writes the file 2 to 4 s after the last change.

### Used settings

A module names each setting that it uses in `uses`, as `MODULE.KEY`. A used setting is a public setting of another loaded module, or a capability of the core, `CORE.<KEY>`.

- If a required used setting is missing, the module does not load. The report gives error 55 with the name.
- A name that starts with `?` is optional. If it is missing, its handle is `GRKIT_NO_SETTING`. The installer does not register a page, a layer, or a filter of that setting.
- A setting of a module that does not load is missing, also when that module comes later in the order. A module that is Off loads stopped, so its settings are not missing.

## Capabilities

The capabilities are public settings of the core. A module uses one by its name in `uses`. The core applies the value, so module code does not run in the hot path. INSTALL installs the firmware changes of a capability only when a loaded module uses it.

| Name | Values | Meaning |
| --- | --- | --- |
| `CORE.TONE` | 0 Off, then n + 1 for installed effect n | The effect of still pictures and the live view. The core installs the effects only when a loaded module uses `CORE.TONE`. |
| `CORE.ASPECT` | 8: 0 native 3:2, 1 XPan 65:24, 2 16:9, 3 4:3, 4 2.35:1, 5 1:1, 6 5:4, 7 7:6 | The aspect of still pictures. |
| `CORE.GRAIN` | 5: 0 Off, 1 weak, 2 medium, 3 strong, 4 extra strong | The grain of recorded pictures, in the native grain stage. The live view and DNG files have no grain. |
| `CORE.GRAIN_SIZE` | 4: 0 Auto, 1 small, 2 medium, 3 large | The grain size of recorded pictures. Auto follows the ISO of the shot. |

- `CORE.TONE`: a page gives only its title, and the core gives the rows: the effects, their icons, the preview, and the Fn adjustments. The value of an effect is valid only during one start, because the effects and their order can change. Keep no value across a restart.
- `CORE.ASPECT`: a page shows only the ratios that the camera offers. The core crops each ratio from the native 3:2, also a ratio that the camera has natively. A layer gets the crop in `Surface.area` and `Surface.box`. `setting_set` with a ratio that the camera does not offer does nothing.

## Registrations

A registration connects a part of the module to an extension point of the core. The build writes the registrations of the manifest into the descriptor.

| Point | Structure | Manifest |
| --- | --- | --- |
| `GRKIT_POINT_MENU_PAGE` (menu.page) | `GrkitPage` | `page` |
| `GRKIT_POINT_OVERLAY_LAYER` (overlay.layer) | `GrkitLayer` | `layers` |
| `GRKIT_POINT_PICTURE_PLANE` (picture.plane) | `GrkitPlaneFilter` | `filters` |

### Pages

A page is a choice page in ADJ: one row for each value of its setting. OK on a row sets the value. The root shows a page only if it has two rows or more, in the order of the modules, which ADJ Order in MENU can change. The installer rejects a page whose number of rows is not the number of values of its setting.

A page of a setting of the module can also give 2 to 5 rows of a native settings page (`GRKIT_SETTING_ROWS`), as `modules/date` does. The first row is the page setting, and the others are settings of the module or used settings, with at most 8 values each. The text of a used setting gives its `values`. INSTALL leaves out the row of an optional used setting that no module has. Fn in ADJ opens this page.

Each module also has a row in the GRKit section of MENU. It opens Enable, then the rows of the settings page, or the row of the page setting for a page without settings rows. See [GRKit section of MENU](architecture.md#grkit-section-of-menu).

### Layers

A layer paints over the still live view. The core calls `paint` for each draw of the live view with the value of the layer setting. It does not call `paint` while the value is 0.

The surface has 720 x 480 pixels of 32 bits. A pixel holds red in its low byte, then green, blue, and alpha in its high byte, so the bytes in memory are R, G, B, A:

| Field | Meaning |
| --- | --- |
| `data` | the address of the pixels. Use `SURFACE_PIXEL(s, x, y)` to address a pixel. |
| `stride` | the distance between rows, in pixels |
| `area` | the capture area that the surface shows: 0 for the full 3:2 area, else the `CORE.ASPECT` value of the crop |
| `box` | the photo area {left, top, right, bottom} |

Layers paint in ascending `order`. A layer with the same order as another paints after the layers of earlier modules. The official layers use these orders:

| Order | Layer |
| --- | --- |
| 10 | the Framing mask |
| 20 | Guides |
| 30 | the Framing ratio label |

The manifest keys `follow_disp`, `full_disp`, and `live_only` set the flags `GRKIT_LAYER_FOLLOW_DISP`, `GRKIT_LAYER_FULL_DISP`, and `GRKIT_LAYER_LIVE_ONLY`. While the page of a layer setting is open in ADJ, the core paints the layer with the highlighted value on the ADJ canvas.

### Plane filters

A plane filter changes the YCbCr planes of each recorded picture. The core calls `apply` for each recorded picture with the value of the filter setting. It does not call `apply` while the value is 0.

| Field | Meaning |
| --- | --- |
| `PictureSet.planes` | the planes of one encode: the main picture, the screennail, and the thumbnail, without duplicates |
| `PictureSet.count` | the number of planes |
| `PictureSet.mono` | set for a monochrome picture |
| `PictureSet.scratch` | `GRKIT_PLANE_SCRATCH` bytes of scratch memory for this call |
| `PictureSet.turns` | the quarter turns clockwise that show the picture upright, from the posture of the camera at the shot: 0, 1, 2, or 3, as EXIF `Orientation` 1, 6, 3, or 8. The planes keep the landscape layout of the sensor. |
| `Plane.y`, `Plane.c` | the addresses of the luma rows and of the interleaved CbCr rows |
| `Plane.width`, `Plane.height`, `Plane.pitch` | the size, and the distance between rows in bytes |
| `Plane.half` | set when there is one CbCr row for every second luma row. `chroma_row(p, y)` gives the CbCr row of luma row y. |
| `Plane.left`, `top`, `right`, `bottom` | the visible area |

Use `GRKIT_BYTES(address)` to address a byte of a plane.

Filters run after the color of the effect, in ascending stage. In one stage, the filters run in the order of the modules.

| Stage | Manifest | Use |
| --- | --- | --- |
| `GRKIT_STAGE_FRAME` (20) | `frame` | frames and borders |
| `GRKIT_STAGE_MARK` (30) | `mark` | marks over the picture, such as the date |

## Services

A module calls the core only through these functions. The installer links each call at power-on.

| Function | Description |
| --- | --- |
| `u32 setting_get(u32 handle)` | The value of a setting. 0 for `GRKIT_NO_SETTING`. |
| `void setting_set(u32 handle, u32 value)` | Set a setting. A value out of range does nothing. |
| `void overlay_label(const Surface *s, u32 label, u32 left, u32 top)` | Draw a label of the core with its top left at (left, top), in the style of the native live-view text. A label that does not fit draws nothing. |
| `void local_time(GrkitDateTime *out)` | The local time: year, month, day, hour, minute, and second. |
| `u32 divide(u32 n, u32 d)` | n / d, rounded down, for d > 0. |
| `u32 timer_start(u32 ms, void (*fire)(u32 arg), u32 arg)` | Call `fire(arg)` once, `ms` milliseconds later, in the App task. Returns a timer, or 0 when no timer is free or the caller is not in the App task. |
| `void timer_cancel(u32 timer)` | Stop a timer that has not fired. |
| `u32 ui_toast(GrkitText text, u32 ms)` | Show a message panel for `ms` milliseconds. See [UI](#ui). |
| `u32 ui_message(GrkitText text, void (*done)(u32 arg), u32 arg)` | Show a message until OK or MENU. |
| `u32 ui_confirm(GrkitText text, void (*done)(u32 executed, u32 arg), u32 arg)` | Ask Execute or Cancel. |
| `u32 ui_progress_open(GrkitText text)` | Show a message with a progress bar. |
| `void ui_progress_set(u32 percent)` | Set the progress bar, 0 to 100. |
| `void ui_progress_close(GrkitText text)` | Close the progress bar, and show `text` for 1.5 s, or close the box when `text` is 0. |
| `u32 ui_open_native(u32 screen, void (*back)(u32 arg), u32 arg)` | Open a native screen of the camera, then return to the GRKit page and call `back(arg)`. See [UI](#ui). |
| `u32 ui_slider(GrkitText text, u32 min, u32 max, u32 value, void (*changed)(u32 value, u32 arg), void (*done)(u32 committed, u32 value, u32 arg), u32 arg)` | Ask for a value from `min` to `max` with the native slider. See [UI](#ui). |

`overlay_label` takes these labels:

| Label | Text |
| --- | --- |
| `LABEL_GRKIT` | the GRKit cue |
| `LABEL_RATIO(area)` | the name of a capture area (`Surface.area`), such as "XPan" |
| `LABEL_DATE` | "DATE", the sign that Date Imprint is on |

`LABEL_TOP` is the top of a label in the second row of the native live-view text. `LABEL_LEFT` is the left edge that lines up with the native icons. A capture area label is at most 80 pixels wide, so a second label of the row starts at `LABEL_NEXT_LEFT` and never covers it.

A module also imports `grkit_self`, its context, which `GRKIT_SETTING` and `GRKIT_USED` use. `grkit_core::grm::EXPORTS` lists the exported functions. A new service is an addition to that list and to `grkit.h`, and it raises the minor ABI version.

## Events

A module subscribes to events with `[events]` in `module.toml` (`GrkitSubscriber`, registration point `GRKIT_POINT_EVENT`). The core calls the handler in the App task, which is the task of the camera UI. So a handler can call the timer and UI services.

| Event | When | `a` | `b` |
| --- | --- | --- | --- |
| `GRKIT_EVENT_SYSTEM_READY` | once after each start, when the shooting screen first shows | 0 | 0 |
| `GRKIT_EVENT_SETTING_CHANGED` | after a setting or a capability changes, in the GRKit menu or through `setting_set` | the handle | the new value |

A handler must return at once. A `setting_set` in a handler sends no second event, so a handler cannot loop. Layers and plane filters run in other tasks, where the timer and UI services refuse each call.

## UI

The UI services show the native message box of the camera. A text is a `GrkitText`: one UTF-16 string for each menu language.

- The box shows only while the GRKit page of ADJ is open. A call returns 1 when the box shows, else 0.
- One box shows at a time. A toast closes when a new box opens. A message, a confirm, or a progress box takes each key until it closes.
- ADJ exit closes the box without a callback.
- OK on a row of a module page commits the value and closes ADJ, as OK on a native value does. If the commit makes the module show a box or open a native screen (in its `SETTING_CHANGED` handler), ADJ stays open on the page, so the box shows.
- `ui_confirm` shows Execute and Cancel, with the cursor on Cancel. Up, Down, Left, and Right move the cursor. OK calls `done(1, arg)` for Execute, else `done(0, arg)`. MENU calls `done(0, arg)`.
- `ui_message` closes on OK or MENU and calls `done(arg)`.
- A progress box takes the keys until `ui_progress_close`. Use a timer to move the bar.
- `ui_slider` shows the text and the native slider. Up and Down move it by one, Left and Right by a tenth of the range, and each move calls `changed(value, arg)`, for example to preview the value. OK calls `done(1, value, arg)`, MENU `done(0, start, arg)` with the value at the start. The range has at most 65536 values.
- `ui_open_native` opens a native screen after the current event. MENU on the screen returns to ADJ, which opens the GRKit page that was open, and then the core calls `back(arg)` (`back` can be 0). The screens: `GRKIT_NATIVE_WHITE_BALANCE` (the white balance list of still mode) and `GRKIT_NATIVE_LCD` (the LCD settings). It returns 0 for an unknown screen, while a message, a confirm, or a progress box shows, or while another screen waits to open.

The UI services work only in an event handler, a timer callback, or a UI callback, which the core calls in the App task. In other code they return 0.

## ABI and compatibility

**ABI version.** `GRKIT_ABI` is 2.0: the major version in the high 16 bits, the minor version in the low 16 bits. The build writes it into the file header and into the descriptor. The installer loads a module whose major version equals that of the core and whose minor version is not larger. A minor version only adds functions and registration types. A major version changes a layout. In 2.0, each text list also has a Korean text, so the installer rejects a module with ABI 1.0 (status 51). Build such a module again with a `text/ko.toml`.

**File format.** A module file is GRM version 1 (`grkit_core::grm`): a header, the code, the data, the relocations, and the imports. The build compiles each source as Thumb code for ARMv7-A with `-Os`. The installer only moves absolute 32-bit words and fills the imports by name hash. The build rejects other relocation types and each undefined symbol that is not a service or `grkit_self`.

**Memory.** The code, with the constant data, goes after the RTOS image and is read-only. The data and BSS go into one script array that all modules share, and INSTALL clears the BSS. The installer runs no module code at power-on: it reads the constant descriptor `grkit_module`.

**Stack.** Module code runs on the stacks of firmware tasks. The build rejects a function whose stack frame is larger than 1536 bytes. It does not check the total stack of nested calls.

## Limits

| Limit | Value | Error when exceeded |
| --- | --- | --- |
| Module files that GRKit lists in both module folders | 10 | GRKit keeps the package modules, then the first other files in the default order. A module of `GRKIT/MODULES` never takes the place of a package module. The report gives the number of the other files. |
| Module ID | 1 to 8 characters `A`-`Z`, `0`-`9`, `_` | build error |
| Code, data, or BSS of one module | 65536 bytes each | build error, or 50 |
| Code of all modules | 1 MiB | 54 |
| Registrations of one module | 16 | 53 |
| Settings of one module | 16 | 53 |
| Used settings of one module | 8 | 53 |
| Setting key | 1 to 15 characters `A`-`Z`, `0`-`9`, `_` | build error, or 53 |
| Values of one setting | 1 to 65536 | build error, or 53 |
| Entries of the settings table: the settings of all modules, 2 for each listed module, and the entries that the core keeps for the effects | 160 | 54 |
| Tiles of one module | 64 | build error, or 53 |
| Tiles of all loaded modules | 128 | 54 |
| Rows of a choice page | 1 to 64 | build error, or 53 |
| Rows of a native settings page | 5 | build error, or 53 |
| Values of a native settings row | 8 | 53 |
| Event subscribers | 1 in each module | 53 |
| Timers of all modules | 8 | `timer_start` returns 0 |
| Units of one text | 64 UTF-16 units | build error, or 53 |
| Pages of all loaded modules | 10 | 54 |
| Layers of all loaded modules | 16 | 54 |
| Plane filters of all loaded modules | 8 | 54 |
| Layer order | 0 to 65535 | build error |
| Stack frame of one function | 1536 bytes | build error |
| Data, BSS, and tables of all modules | 65536 words in one script array | 54 |

## Report errors

`GRKIT/REPORT.TXT` gives one line for each listed module, such as `HELLO.GRM  not loaded: error 51, it needs another version of GRKit`. The GRKit section of MENU shows the same number after the module name, such as `HELLO (51)`.

| Error | Report text | Cause |
| --- | --- | --- |
| 50 | it is not a module file that this GRKit reads | The file cannot be read, or its magic, format version, sizes, or relocations are not valid. Also when the file name is not `<ID>.GRM` for the ID in the file. |
| 51 | it needs another version of GRKit | The ABI major version is not that of the core, or the minor version is larger. |
| 52 | it calls a function that GRKit does not have: `<name>` | An import names no service of this core. |
| 53 | its description is not valid | The descriptor breaks a rule: an unknown point or flag, a count over its limit, a bad setting key, a text that is too long, a page whose rows do not match its setting, or a reference outside the module. |
| 54 | the modules need more memory than GRKit has | The module does not fit in the code area, the script array, the settings table, the tile slots, or the page, layer, or filter slots that earlier modules left. |
| 55 | it uses a setting that no module has: `<name>` | A required used setting is not a capability, a public setting of a loaded module, or a setting of this module. |

Other lines:

| Report text | Meaning |
| --- | --- |
| `loaded` | The module loaded. |
| `loaded, stopped: Enable is Off in the GRKit section of MENU` | The module loaded, but Enable is Off, so the core calls none of its code. On applies at once. |
| `loaded, stopped: GRKit is Off in the GRKit section of MENU` | The module loaded, but the GRKit switch is Off, so the core calls none of its code. On applies at once. |
| `not loaded: safe mode` | GRKit is in the safe mode, so no module loads. See [Report and safe mode](architecture.md#report-and-safe-mode). |

When a module does not load, the other modules still load. A module that requires a setting of it gets error 55. A page, a layer, or a filter of an optional setting of it is not registered.
