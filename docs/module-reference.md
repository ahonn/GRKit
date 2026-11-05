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
| `color_only` | boolean | `false` | The module loads only on a camera with a color sensor. GR IV Monochrome does not load it, and the GRKit section of MENU does not list it. Needs ABI 2.4. |
| `settings` | array of tables | `[]` | The settings of the module. At most 16. |
| `page` | table | none | The menu page. A module has at most one. |
| `layers` | array of tables | `[]` | The live-view layers. |
| `indicators` | array of tables | `[]` | The viewfinder status icons. |
| `filters` | array of tables | `[]` | The plane filters. |

A manifest gives at least one page, layer, indicator, filter, or event subscriber. Each entry is one registration, and a module has at most 16 registrations.

A setting reference is a key of `settings`, or a name of `uses` without its `?`.

### settings

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `key` | string | required | 1 to 15 characters `A`-`Z`, `0`-`9`, or `_`. Unique in the module. The full name is `<id>.<key>`. |
| `values` | integer | required | The number of values, 1 to 65536. The values are 0 to `values` - 1. |
| `default` | integer | `0` | The initial value, from 0 to `values` - 1. Used when no valid saved value exists. |
| `saved` | boolean | `false` | The state file keeps the value, including 0. |
| `public` | boolean | `false` | Other modules can use the setting. |
| `color_only` | boolean | `false` | The rows of the setting show only on a camera with a color sensor. On GR IV Monochrome, the installer leaves them out of each page, and the setting keeps its value. It cannot be the first row of a page or of a group child. Needs ABI 2.4. |

### page

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `kind` | string | `"choice"` | `"choice"` lists setting values. `"settings"` opens native settings directly. `"group"` lists children, each with its own list of values. |
| `setting` | string | required for `choice` | The setting of a choice page. Omit it for `settings` and `group`. |
| `off_row` | boolean | `false` | Row 0 shows the core Off icon instead of the Off icon of the page. |
| `settings` | array of strings | `[]` | Up to 5 native settings rows (`GRKIT_SETTING_ROWS`). A choice page starts with its page setting. A settings page needs at least one row. |
| `options` | boolean | `false` | The settings page of the module in the GRKit section of MENU has an Options row, which turns each value of the page on or off in ADJ, or each effect for a page of `CORE.TONE` (`GRKIT_CHOICE_OPTIONS`). A page of a setting with options needs `off_row` and at most 16 values. Without room on the settings page, its first rows leave it. |
| `children` | array of tables | `[]` | A group page has 1 to 4 children, in display order. Other page kinds have none. |
| `off_icon` | table | none | Optional [icon source](#icon-sources) of row 0 (Off) of a choice page without `off_row`. Without it, row 0 shows tile 0. The root row shows tile 0 while the setting is 0. Other page kinds have none. |
| `fn_cycle` | table | none | `{ value = n, setting = "KEY" }`: in ADJ, Fn on the row of value `n` sets the setting to its next value, and to 0 after the last one. ADJ stays on the row, and the live view shows the change at once. Fn on the other rows opens the settings as before. `n` is a value of the page other than Off. The setting is a setting reference other than the page setting, with at least 2 values. Only a choice page whose rows come from the module has it. Needs ABI 2.5. |

A `settings` page needs a module tile and has no `setting`, `off_row`, or `options`. Its text gives `title` and `settings.<KEY>.label` and `values` for each row. It has no `rows` or `summaries`. The first row must be available for the page to load. Later optional rows are omitted when unavailable.

```toml
[page]
kind = "settings"
settings = ["BLOOM", "HALATION"]
```

A `group` page has no `setting`, `settings`, `off_row`, or `options`. Each `[[page.children]]` table has these fields:

| Key | Type | Description |
| --- | --- | --- |
| `key` | string | A unique key with 1 to 15 characters `A`-`Z`, `0`-`9`, or `_`. It identifies the child title in the text files. |
| `settings` | array of strings | The child's 1 to 5 native settings rows. The first setting has 2 to 8 values: 0 is Off, and the other values turn the child on. It must be available and cannot be an optional use. |
| `icon` | table | Optional [icon source](#icon-sources). Without it, child n uses tile n + 1. |

This example uses two settings declared elsewhere in the manifest, each with the values Off, Weak, Medium, and Strong:

```toml
[page]
kind = "group"

[[page.children]]
key = "BLOOM"
settings = ["BLOOM"]
icon = { setting = "BLOOM", first_tile = 1 }

[[page.children]]
key = "HALATION"
settings = ["HALATION"]
icon = { setting = "HALATION", first_tile = 5 }
```

The text files give `title`, `children.<KEY>.title`, and the label and values of each setting. A group has no choice `rows` or `summaries`. Tile 0 identifies the group. Without an explicit `icon`, tiles 1 to the child count identify its children in declaration order.

The generator registers the group first, then its child settings pages. Each child has `GRKIT_PAGE_MENU_ONLY`, so only the group appears in the ADJ root. A group and its children consume one page slot each from the shared limit of 10 loaded pages.

A page of `CORE.TONE` or `CORE.RECIPE` has no `off_row` and no `settings`. Its text has only `title`. A page of `CORE.TONE` needs no tiles. If tile 0 exists, the root row shows that tile while the setting is Off. Otherwise, the root row shows the Off icon. With `options = true`, Options of a page of `CORE.RECIPE` lists each recipe that is on and chooses the ones that ADJ shows.

### layers

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `paint` | string | required | The C function `void paint(const Surface *s, u32 value)`. |
| `setting` | string | none | The setting whose value the layer paints: a setting reference. Without it, the value is always 1. |
| `order` | integer | required | The paint order, 0 to 65535. |
| `follow_disp` | boolean | `false` | The layer is off while DISP hides the native information. |
| `full_disp` | boolean | `false` | The layer is on only while DISP shows the full information. |
| `live_only` | boolean | `false` | The layer does not paint on the ADJ canvas. |
| `enabled` | condition or `{ all = [...] }` | none | Optional setting conditions. All must match before the core treats the layer as active. |

A layer has `follow_disp` or `full_disp`, not both.
The optional `enabled` field uses the same conditions as an indicator, with 1 to 4 conditions.
An omitted `enabled` field adds no condition.
The setting value, display flags, and conditions must all permit the layer to draw.
For example, `enabled = { setting = "DISP", equals = 0 }` selects a layer when `DISP` is zero.

On the ADJ canvas, a condition on the previewed setting reads the highlighted value.
Other conditions read committed values.

### indicators

An indicator supplies a status icon without a drawing callback. The viewfinder service owns its placement, display policy, and refresh.

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `id` | string | required | A unique identifier within this module: 1 to 32 lowercase letters, digits, or underscores. |
| `placement` | string | required | `native_slot` or `status_row`. |
| `slot` | string | none | Required for `native_slot`: `image_control`. Omit it for `status_row`. |
| `enabled` | table | required | A setting condition, or `all = [condition, ...]` with 1 to 4 conditions. |
| `icon` | table | required | An [icon source](#icon-sources). |

A condition has `setting` and exactly one of `equals` or `not_equals`. All conditions must hold for the indicator to show. The core also hides indicators of stopped modules or when GRKit is off.

```toml
[[indicators]]
id = "framing"
placement = "status_row"
enabled = { all = [{ setting = "CORE.ASPECT", not_equals = 0 }, { setting = "LABEL", equals = 0 }] }
icon = { setting = "CORE.ASPECT", label = "ratio" }
```

The status row uses the module order, then declaration order. Disabled icons leave no gap. It supports at most 6 registered icons across loaded modules. It shows only on the full-information still viewfinder and uses screen coordinates independently of the current photo area.

The `image_control` slot supports one registered indicator. It returns the current native icon when the indicator is disabled or cannot resolve its icon. Its visibility follows the native still-viewfinder position. It does not change the camera's Image Control setting or playback icons.

Indicators use committed settings, not ADJ highlight previews. Their icons indicate enabled settings, including effects without a live preview. Registration conflicts and capacity failures are reported during installation.

### Icon sources

Indicators, group children, and the `off_icon` of a choice page accept the same `icon` table:

| Form | Meaning |
| --- | --- |
| `{ tile = 2 }` | The fixed module tile at index 2. |
| `{ setting = "STYLE" }` | Module tile n for setting value n. |
| `{ setting = "STRENGTH", first_tile = 1 }` | Module tile 1 + n for setting value n. |
| `{ setting = "CORE.TONE" }` | The core icon of the committed Film Look. |
| `{ label = "DATE" }` | A core live-view label, by its text. Status-row indicators only. |
| `{ setting = "STYLE", label = "DATE" }` | The core label n after `DATE` for setting value n. Status-row indicators only. |
| `{ setting = "CORE.ASPECT", label = "ratio" }` | The core label of the committed Framing ratio. Status-row indicators only. |

A core label is the native live-view text, as in the focus distance `2.5m`. The labels are, in order, `XPan`, `16:9`, `4:3`, `2.35`, `1:1`, `5:4`, `7:6`, `DATE`, `YMD`, `YMD HM`, `MDY`, `DMY`, and `DMY HM`. `YMD`, `MDY`, and `DMY` are as high as a ratio. `YMD HM` and `DMY HM` show `HM` on a second line and are as high as a status-row icon. With a setting, every value needs a label at or after the named one. A label takes its own width in the status row; an icon takes 44 pixels. The row ends before x = 580. When the camera shows its own information in the same row, such as the white balance shift or the dial guide of Snap mode, the row moves to the free space next to it. The row waits while the load cue shows. An indicator that does not fit is not shown, nor are the indicators after it.

Do not combine `tile` with `setting` or `first_tile`. A tile mapping must cover every value of its setting. `CORE.TONE` uses the core's effect icons and has no tile offset.

An icon source does not decide whether a feature is enabled. A viewfinder indicator uses its `enabled` condition for that. For example, Glow's Bloom indicator hides while Bloom is Off, and its ADJ child shows the Off tile.

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
| `children.<KEY>.title` | string | The title of a group child. The ADJ group and the module page in MENU use this title for the child row. |

A choice root row shows `<title><separator><summary>`. A group root row shows `<title><separator>` and the titles of the children that are on, joined by a slash, or Off. A settings root row shows only the title. Each text, and the text of the root row, has at most 64 UTF-16 units. For a choice page of a used setting, the build counts `rows` in `text/en.toml`. The installer checks this count against the setting.

For example, these tables name a group child and its setting:

```toml
[children.BLOOM]
title = "Bloom"

[settings.BLOOM]
label = "Strength"
values = ["Off", "Weak", "Medium", "Strong"]
```

## Tiles

Built-in artwork follows the [icon design rules](icon-guidelines.md).

A tile is a 40 x 40 PNG file. A choice page needs at least one tile for each setting value. Tile n shows value n on the choice row and on the root row. Tile 0 also identifies the module. An `off_icon` replaces tile 0 on the Off row. A settings page needs only tile 0. A group page needs tile 0 and its child icons, including every tile named by an explicit icon source. A child icon of its first setting needs a tile for Off, value 0.

The build stores the tiles after the module file, not in its code. INSTALL copies them to icon slots after the effect icons, so they take no module code memory and do not count toward the 65536-byte code limit. All loaded modules have at most 128 tiles together.

## Settings

The core stores each setting from 0 to `values` - 1. Without a valid saved value, a setting starts at `default`. A module reads and changes a value only through `setting_get` and `setting_set`. Read a setting each time that you use it, because the menu and other modules can change it.

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

- A new ID or key starts at `default` (0 if omitted). A valid saved value overrides it, including 0.
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
| `CORE.ASPECT` | 8: 0 Off, 1 XPan 65:24, 2 16:9, 3 4:3, 4 2.35:1, 5 1:1, 6 5:4, 7 7:6 | The selected crop or composition ratio, according to `CORE.ASPECT_MODE`. |
| `CORE.ASPECT_MODE` | 2: 0 Crop, 1 Guides Only | Whether the selected ratio crops JPEG photos or only marks composition in the live view. The default is Crop. |
| `CORE.CROP` | 6: 0 Off, 1 35 mm, 2 40 mm, 3 50 mm, 4 70 mm, 5 90 mm | A focal-length crop of the lens, according to `CORE.CROP_MODE`. The focal lengths are those of a 28 mm lens. GR IIIx has the same crops of its 40 mm lens: 50, 57, 71, 100, and 128 mm. |
| `CORE.CROP_MODE` | 2: 0 Crop, 1 Frame Only | Whether the camera crops photos to the selected focal length or the live view only shows its frame. The default is Crop. |
| `CORE.GRAIN` | 5: 0 Off, 1 weak, 2 medium, 3 strong, 4 extra strong | The grain of recorded pictures, in the native grain stage. The live view and DNG files have no grain. |
| `CORE.GRAIN_SIZE` | 4: 0 Auto, 1 small, 2 medium, 3 large | The grain size of recorded pictures. Auto follows the ISO of the shot. |
| `CORE.RECIPE` | none | The recipes: saved sets of the other capabilities that OK applies in one step. |

- `CORE.TONE`: a page gives only its title, and the core gives the rows: the effects, their icons, the preview, and the Fn adjustments. The value of an effect is valid only during one start, because the effects and their order can change. Keep no value across a restart.
- `CORE.RECIPE`: a page gives only its title and four tiles: the module, Undo, Save, and a recipe without an effect icon or a native Image Control icon. The core gives the rows: Undo, the recipes that ADJ shows, and Save Settings. In MENU, the core gives the page of the module: Enable, Options (with `options = true`), Recipes, which lists the recipes that are on and opens the page of one with Apply, Rename, and Delete, and Save Settings. A recipe holds and sets only the capabilities that other loaded modules use. `setting_get` gives 0, and `setting_set` does nothing.
- `CORE.ASPECT`: a page shows only the ratios that the camera offers. `setting_set` with an unsupported ratio does nothing. Selecting a nonzero ratio sets the native aspect to 3:2 in both modes. Crop mode crops JPEG photos to that ratio. Guides Only displays the composition frame without applying that crop. Off clears the selected ratio and preserves the current native aspect.
- `CORE.ASPECT_MODE`: Crop changes JPEG dimensions. Guides Only shows the selected ratio in the native 3:2 live view without setting a recorded crop. A later change to another native aspect hides this composition frame. Changing only the mode preserves the native aspect and the Framing mask setting.

- `CORE.CROP`: a page shows only the crops that the camera offers, and the core gives the texts of GR IIIx. In Crop mode, in native 3:2 without a `CORE.ASPECT` ratio, the camera crops JPEG and DNG to the focal length, one pixel each, and Exif names it. The live view magnifies to 35 or 50 mm, and shows the frame of 40 mm in the 35 mm view and of 70 and 90 mm in the 50 mm view. The camera's crop button steps through the crops. With a `CORE.ASPECT` ratio, in another native aspect, or in Frame Only mode, the live view shows the frame of the crop and photos are not cropped.

When Framing crops a shot, DNG defaults to native 3:2 and preserves the camera's digital crop setting. RAW editors that apply the DNG default crop show this 3:2 area. Embedded previews can still show the selected composition.

## Registrations

A registration connects a part of the module to an extension point of the core. The build writes the registrations of the manifest into the descriptor.

| Point | Structure | Manifest |
| --- | --- | --- |
| `GRKIT_POINT_MENU_PAGE` (menu.page) | `GrkitPage` | `page` |
| `GRKIT_POINT_OVERLAY_LAYER` (overlay.layer) | `GrkitLayer` | `layers` |
| `GRKIT_POINT_VIEWFINDER_INDICATOR` (viewfinder.indicator) | `GrkitIndicator` | `indicators` |
| `GRKIT_POINT_PICTURE_PLANE` (picture.plane) | `GrkitPlaneFilter` | `filters` |

### Pages

A `choice` page lists one row for each value of its setting in ADJ. OK on a row sets the value. ADJ shows a choice page when it has at least two rows. ADJ Order in MENU sets the module order. The installer rejects a page whose number of rows is not the number of values of its setting.

A page of a setting of the module can also give 2 to 5 rows of a native settings page (`GRKIT_SETTING_ROWS`), as `modules/date` does. The first row is the page setting, and the others are settings of the module or used settings, with at most 8 values each. The text of a used setting gives its `values`. INSTALL leaves out the row of an optional used setting that no module has. Fn in ADJ opens this page.

A `settings` page shows one ADJ entry with its title and module tile. OK, Right, Fn, or a touch opens the native settings directly. Each row changes only its own setting. MENU shows Enable followed by the same rows.

A `group` page shows one ADJ entry. OK opens an icon list of its children. A selection mark identifies each child that is on. OK on a child opens a list of the values of its first setting, with the value texts of its settings page and the child icon of each value. OK on a value sets it and closes ADJ, as on a choice page. MENU returns from the value list to the child's row of the group.

Fn on a child opens the child's native settings page. Returning to ADJ restores the selected child.

MENU shows the module's Enable row and one row for each child, with the value of the child's first setting. Each child row opens the same native settings page as ADJ. Returning from that page restores the module page. The module's Enable setting controls the whole module.

Each module also has a row in the GRKit section of MENU. It opens Enable, then the rows of the settings page, or the row of the page setting for a page without settings rows. See [GRKit section of MENU](architecture.md#grkit-section-of-menu).

### Layers

A layer paints over the still live view. The core calls `paint` for each draw of the live view with the value of the layer setting. It skips a layer whose setting is zero or whose enabled conditions do not match.

The surface has 720 x 480 pixels of 32 bits. A pixel holds red in its low byte, then green, blue, and alpha in its high byte. The bytes in memory are R, G, B, A:

| Field | Meaning |
| --- | --- |
| `data` | the address of the pixels. Use `SURFACE_PIXEL(s, x, y)` to address a pixel. |
| `stride` | the distance between rows, in pixels |
| `area` | the displayed area: 0 for full 3:2, otherwise a ratio value from `CORE.ASPECT`. See the layer rules below. |
| `box` | the displayed area {left, top, right, bottom}, with right and bottom excluded |

A layer receives the selected framing area, in Crop mode and in Guides Only mode.
Without a framing area, it receives the photo area of the camera's native aspect ratio.
A layer whose setting is `CORE.ASPECT` receives area 0 without a framing area.
A frame of `CORE.CROP` is the center of that area. A layer whose setting is `CORE.CROP` receives the frame as `box` and the crop as `area`, or area 0 without a frame, such as a crop that the live view already shows. The other layers, except those of `CORE.ASPECT`, receive the frame as `box`.

In ADJ, each layer previews the highlighted ratio.

Layers paint in ascending `order`. A layer with the same order as another paints after the layers of earlier modules. The official layers use these orders:

| Order | Layer |
| --- | --- |
| 10 | the Framing mask |
| 15 | the Crop mask |
| 20 | Guides |

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
| `void picture_white(const PictureSet *set, u8 white[3])` | The R, G, B white of the effect of a recorded picture: the levels that its brightest light reaches. 255 each without an effect. A color effect ends below 255. Since ABI 2.1. |
| `void picture_shot(const PictureSet *set, GrkitShot *shot)` | The shooting data of a recorded picture, as its EXIF records it: the exposure (APEX values, the exposure value of the scene, the f-number, the ISO speed, the exposure time), the exposure compensation, the focal length, the program, the metering, the flash, and the capture time. Set `shot->size` to `sizeof(GrkitShot)` first. `known` has a bit for each group of fields that holds a value. Since ABI 2.1. |
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
- OK on a row of a choice page commits the value and closes ADJ, as OK on a native value does. A commit can make the module show a box or open a native screen in its `SETTING_CHANGED` handler. ADJ then stays open on the page, so the box shows.
- `ui_confirm` shows Execute and Cancel, with the cursor on Cancel. Up, Down, Left, and Right move the cursor. OK calls `done(1, arg)` for Execute, else `done(0, arg)`. MENU calls `done(0, arg)`.
- `ui_message` closes on OK or MENU and calls `done(arg)`.
- A progress box takes the keys until `ui_progress_close`. Use a timer to move the bar.
- `ui_slider` shows the text and the native slider. Up and Down move it by one. Left and Right move it by a tenth of the range. Each move calls `changed(value, arg)`, for example to preview the value. OK calls `done(1, value, arg)`, MENU `done(0, start, arg)` with the value at the start. The range has at most 65536 values.
- `ui_open_native` opens a native screen after the current event. MENU on the screen returns to ADJ, which opens the GRKit page that was open, and then the core calls `back(arg)` (`back` can be 0). The screens: `GRKIT_NATIVE_WHITE_BALANCE` (the white balance list of still mode) and `GRKIT_NATIVE_LCD` (the LCD settings). It returns 0 for an unknown screen, while a message, a confirm, or a progress box shows, or while another screen waits to open.

The UI services work only in an event handler, a timer callback, or a UI callback, which the core calls in the App task. In other code they return 0.

## ABI and compatibility

**ABI version.** `GRKIT_ABI` is 2.6: the major version in the high 16 bits, the minor version in the low 16 bits. The build writes it into the file header and into the descriptor. The installer loads a module whose major version equals that of the core and whose minor version is not larger. A minor version adds capabilities without changing existing contracts. A major version can change an existing layout.

In 2.0, each text list also has a Korean text. The installer rejects a module with ABI 1.0 (status 51). Build such a module again with a `text/ko.toml`.

ABI 2.1 adds `picture_white`, `picture_shot`, setting defaults, and the settings and group page kinds. `GrkitSetting` appends `default_value`, and `GrkitPage` appends the group `children` pointer. The installer accepts older 16-byte settings and 40-byte pages. Older settings have default 0. A group needs the 44-byte page descriptor and registers each child separately.

ABI 2.2 adds `GrkitIndicator`, `GrkitCondition`, and `GrkitIcon`. `GrkitPage` appends an optional `icon` pointer after `children`. The installer still accepts 40-byte and 44-byte pages. Their icons keep the existing tile rules. `GrkitLayer` and `Surface` keep their existing layouts.

ABI 2.3 appends `condition_count` and `conditions` to `GrkitLayer`, increasing its descriptor from 20 to 28 bytes.
The installer still accepts 20-byte layers, which have no enabled conditions.
A 28-byte layer without conditions has count zero and a null pointer.
A conditional layer has 1 to 4 `GrkitCondition` entries.
The installer rejects layer descriptors of 21 to 27 bytes.
`Surface` keeps its existing layout.

ABI 2.3 also moves the tiles out of the code. The header word at offset 60, which was reserved and 0, gives the number of tiles that follow the import names. The descriptor then gives `tile_count` and a null `tiles` pointer. The installer still accepts tiles in the code, through a `tiles` pointer and a header word of 0. It rejects tiles after the file of a module of ABI 2.2 or earlier, tiles in both places, and a `tile_count` that differs from the header.

ABI 2.3 adds the icon kind `GRKIT_ICON_LABEL` (3) for status-row indicators: a core live-view label, `first_tile` + n for setting value n, or `first_tile` without a setting. The installer rejects it for pages and the Image Control slot, and when the labels of the setting values do not exist.

ABI 2.4 adds the setting flag `GRKIT_SETTING_COLOR_ONLY` (4). The installer of an earlier core rejects a setting with this flag.

ABI 2.4 also appends `flags` to `GrkitModule`, increasing its descriptor from 40 to 44 bytes. The flag `GRKIT_MODULE_COLOR_ONLY` (1) makes a module color-only. The installer still accepts 40-byte descriptors, which have no flags, and rejects an unknown flag.

ABI 2.5 appends the Fn value and setting of `fn_cycle` to `GrkitPage`, increasing its descriptor to 56 bytes.

ABI 2.6 adds the capabilities `CORE.CROP` and `CORE.CROP_MODE`. `Surface` keeps its layout.

**File format.** A module file is GRM version 1 (`grkit_core::grm`): a header, the code, the data, the relocations, the imports, and the tiles. The build compiles each source as Thumb code for ARMv7-A with `-Os`. The installer only moves absolute 32-bit words and fills the imports by name hash. The build rejects other relocation types and each undefined symbol that is not a service or `grkit_self`.

**Memory.** The code, with the constant data, goes after the RTOS image and is read-only. The tiles stay out of it: INSTALL copies them from the file to the icon slots. The data and BSS go into one script array that all modules share, and INSTALL clears the BSS. The installer runs no module code at power-on: it reads the constant descriptor `grkit_module`.

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
| Children of a group page | 1 to 4 | build error, or 53 |
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
| `loaded, stopped: Enable GRKit is Off in the GRKit section of MENU` | The module loaded, but Enable GRKit is Off, so the core calls none of its code. On applies at once. |
| `not loaded: safe mode` | GRKit is in the safe mode, so no module loads. See [Report and safe mode](architecture.md#report-and-safe-mode). |
| `not loaded: it needs a camera with a color sensor` | The module is `color_only`, and the camera is GR IV Monochrome. The GRKit section of MENU does not list it. |

When no loaded module uses the effects, as on GR IV Monochrome without Film Look, each effect line says `not used: no loaded module uses effects`.

When a module does not load, the other modules still load. A module that requires a setting of it gets error 55. A page, a layer, or a filter of an optional setting of it is not registered.
