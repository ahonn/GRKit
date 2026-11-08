# Writing a module

A module adds one feature to GRKit. It is one file, `<ID>.GRM`, that runs on each supported camera.
This guide shows how to write, test, build, and install a module. [module-reference.md](module-reference.md) gives each key, limit, and error number.

The official modules in `modules/` are complete examples:

| Folder | What it shows |
| --- | --- |
| `modules/guides` | an own setting, a choice page, and a layer |
| `modules/framing` | a page, mask layer, and status icon of the capability `CORE.ASPECT` |
| `modules/date` | three settings, a page, a status icon, and a plane filter |
| `modules/glow` | independent Enable and Strength settings, strength icons shared by ADJ and the viewfinder, and a frame filter |
| `modules/grain` | a module with only data: a page and status icon of `CORE.GRAIN` |
| `modules/film-look` | a page of `CORE.TONE` and an indicator in the native Image Control position |

## Prerequisites

- Rust 1.92 or later, to build the `grkit` command.
- Clang with ARM support. Apple Clang works. The build does not need an ARM libc or sysroot.
- LLD as `ld.lld`. The Rust toolchain has one at `$(rustc --print sysroot)/lib/rustlib/<host>/bin/gcc-ld/ld.lld`.
- A host C compiler as `cc`, for the host tests.

Install the command, then check the ARM toolchain:

```sh
cargo install --path crates/grkit-cli --locked
grkit doctor            # add --lld <path> if ld.lld is not on PATH
```

If you give `--lld` or `--clang` to `grkit doctor`, give the same option to `grkit module build`.

## Create a module folder

A module ID has 1 to 8 characters: `A`-`Z`, `0`-`9`, or `_`. The ID is also the file name. Select an ID that no other module on the card has.

```sh
grkit module new --id HELLO --out hello
```

The new folder holds a working module. It has a setting `MODE` (Off, On), a choice page for it, and a layer that draws a white square while `MODE` is On. It has the manifest `module.toml`, the C source and its test, the page text in each menu language, and the tiles. Change it one part at a time, and run the test after each change.

## Describe the module

`module.toml` tells the build what the module gives the core. The build turns it into the descriptor of the module, so you do not write the descriptor in C. This is `modules/guides/module.toml`:

```toml
id = "GUIDES"
version = 1
order = 30
sources = ["guides.c"]
# Tile n shows value n. Tile 0 is the module tile, for Off at the root.
tiles = [
    "tiles/0-module.png",
    "tiles/1-thirds.png",
    "tiles/2-center.png",
    "tiles/3-golden.png",
    "tiles/4-diagonals.png",
    "tiles/5-thirds-center.png",
    "tiles/6-spiral.png",
    "tiles/7-symmetry.png",
]

# 0 Off, 1 thirds, 2 center, 3 golden ratio, 4 diagonals, 5 thirds and center,
# 6 golden spiral, 7 dynamic symmetry.
[[settings]]
key = "STYLE"
values = 8
saved = true
public = true

[page]
setting = "STYLE"
off_row = true

[[layers]]
paint = "guides_paint"
setting = "STYLE"
order = 20
follow_disp = true
```

- `order`: the default place in ADJ and in MENU, after Film Look (10) and Framing (20).
- `[[settings]]`: the setting `GUIDES.STYLE` has 8 values. The state file keeps it (`saved`), and other modules can use it (`public`).
- `[page]`: a choice page of `STYLE` in ADJ and in MENU, with one row for each value.
- `[[layers]]`: the core calls `guides_paint` for each draw of the live view while `STYLE` is not 0.

A module can also use a setting of another module or a capability of the core, such as `uses = ["CORE.ASPECT"]`. A plane filter (`[[filters]]`) changes each recorded picture, as the date of `modules/date` does. A manifest must give at least one page, layer, indicator, filter, or event subscriber. See [module.toml](module-reference.md#moduletoml).

## Add a viewfinder status icon

An indicator declares its visibility and icon. The core handles its position, DISP visibility, and removal when disabled. It does not call module code to draw an indicator.

```toml
[[indicators]]
id = "bloom"
placement = "status_row"
enabled = { setting = "BLOOM", not_equals = 0 }
icon = { setting = "BLOOM", first_tile = 8 }
```

This example shows tiles 9, 10, and 11 for the strengths Weak, Medium, and Strong, values 1 to 3. The `enabled` condition hides the icon while Bloom is Off, value 0. The ADJ group child of Bloom shows the same values with its list tiles 1 to 4, Off included. Status-row icons are drawn for the live view, without a frame, and text uses the core live-view labels, as the Framing ratio does ([icon guidelines](icon-guidelines.md)).

Use `status_row` for an additional icon. Use `native_slot` with `slot = "image_control"` to provide the native still-viewfinder icon, as Film Looks does. When that indicator is disabled, the core restores the current native icon. Guides needs no indicator because the lines already show its state.

Indicators reflect committed settings. The status row shows only in the full-information still viewfinder. It does not appear in ADJ previews, video, or playback. An enabled icon does not mean that the effect has a live preview.

## Write the text and the tiles

A page has text in the five menu languages: `text/en.toml`, `text/zh-Hans.toml`, `text/zh-Hant.toml`, `text/ja.toml`, and `text/ko.toml`.

```toml
title = "Guides"
rows = ["Off", "Thirds", "Center", "Golden Ratio", "Diagonals", "Thirds + Center", "Golden Spiral", "Dynamic Symmetry"]
```

The tiles are 40 x 40 PNG files. Tile n shows value n of the page setting, and tile 0 is also the module tile.

## Write the C code

A module source includes only `grkit.h`, the module SDK in `runtime/sdk/`, and the generated `grkit_module.h`. `grkit_module.h` defines a handle for each setting: `SETTING_<KEY>`, and `USED_<MODULE>_<KEY>` for each used setting.

```c
#include "grkit.h"
#include "grkit_module.h"

/* A layer: called for each draw of the live view with the value of its setting. */
void hello_paint(const Surface *s, u32 value);
/* A plane filter: called for each recorded picture with the value of its setting. */
void hello_apply(PictureSet *set, u32 value);
```

The core does not call a layer or a filter while the value of its setting is 0. Read other settings with `setting_get(SETTING_<KEY>)` each time that you use them, because the menu can change them at any time.

The build or the installer rejects a module that breaks these rules:

- Call only the services that `grkit.h` declares. Do not name a firmware address or symbol: the same file runs on each camera.
- Do not divide by a variable with `/` or `%`, and do not clear or copy a large array in one statement. The compiler calls helpers for them, which the core does not export. Use `divide(n, d)` and loops.
- Keep each stack frame at 1536 bytes or less. Module code runs on the stacks of firmware tasks.

A module has no init function. Static variables are writable, and constant data is read-only. To run code when GRKit is ready or when a setting changes, add an event handler:

```toml
[events]
handle = "hello_event"
on = ["system_ready", "setting_changed"]
```

The handler runs in the App task, so it can start a timer and show a message box. See [Services](module-reference.md#services), [Events](module-reference.md#events), and [UI](module-reference.md#ui).

## Test the module on the computer

`grkit module test` compiles the module with the host compiler and runs `test.c` against a harness that stands in for the core. `runtime/sdk/grkit_test.h` declares the harness: a test surface, plane memory, the settings, the time, the timers, and the UI calls.

```sh
grkit module test --dir hello
```

The command prints `"test": "passed"`, or the failed assertion and exit status 1. A module without C sources has no test, and the command checks only the manifest and the text. A host test checks the logic and the pixels, not the behavior on a camera.

## Build the module file

```sh
grkit module build --dir hello --out HELLO.GRM      # add --lld <path> if needed
```

The output file and the objects folder (`HELLO.objects`) must not exist. The build stops with a message for each rule that the module breaks, for example:

```text
grkit: The module uses __aeabi_uidiv, which the core does not export (runtime/sdk/grkit.h)
```

The installer loads a file only if its name is `<ID>.GRM`.

## Install the module on a card

The card must already hold a GRKit package for the camera.

1. Copy `HELLO.GRM` into `GRKIT/MODULES` on the card.
2. Power on the camera, and wait until ADJ shows the GRK entry.
3. Power off the camera, and open `GRKIT/REPORT.TXT` on the computer.

The report gives one line for each module file:

```text
HELLO.GRM  not loaded: error 55, it uses a setting that no module has: GUIDES.STYLE
```

[Report errors](module-reference.md#report-errors) gives the meaning of each error number. To turn a module off or move its page, use its row and ADJ Order in the GRKit section of MENU. To remove a module, delete its file.

## If the camera stops at power-on

Turn the module off with its **Enable** row in the GRKit section of MENU. If the camera stops before MENU opens, put an empty file `GRKIT/SAFE` on the card: no module loads then. Remove the module, then delete `GRKIT/SAFE`. See [Report and safe mode](architecture.md#report-and-safe-mode).

## Measure the CPU use on the camera

1. Put an empty file `GRKIT/PROFILE` on the card.
2. Power on the camera, and do what you want to measure, such as keeping the live view open or taking photos.
3. Power off the camera, and open `GRKIT/PROFILE.LOG`. Delete `GRKIT/PROFILE` to stop the profile.

Every 10 seconds, the file gets a period with the number of live-view frames, then a line for each point that ran:

```text
period <number> frames <live-view frames>
<point> <calls> <cycles> <cycles of the largest call>
<MODULE> <paint|filter|call> <calls> <cycles> <cycles of the largest call>
```

The core points include the live-view canvas (`frame.bind`, `frame.prepare`, `frame.paint`), the picture (`picture.post`, `picture.encode`), and `menu.icon`. On GR III, `picture.grain` measures the grain noise of a GRKit shot. `picture.grain` counts microseconds instead of cycles. `frame.paint` includes the module layers, and `picture.encode` includes the plane filters.

The profile counts the first eight modules of the list. The profile does not count the Image Control processing of each frame. It writes the card every 10 seconds, so do not use it to measure battery life.
