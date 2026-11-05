# Architecture

GRKit adds features to Ricoh GR cameras without changing the firmware on the camera.
A package on the SD card installs private code into RAM at each power-on, and hooks connect that code to the running RTOS.
This document describes the parts and the approach. The source code and its comments hold the addresses, formats, and limits.

## Overview

```text
 developer computer / browser            camera
 ────────────────────────────            ──────────────────────────────────────────
 grkit build    ──► runtime build        Linux:  loader script ──► INSTALL
 grkit module   ──► GRM modules                     │  checks firmware, links modules,
 grkit effects  ──► GRF effects                     │  writes records, icons, hooks
 grkit package  ──► GRKit.zip  ── SD card ──►       ▼
 site (wasm)    ──► GRKit.zip                RTOS:   hooks ──► core ──► modules
```

There are three execution domains:

- Rust tools run on the developer computer or, through WebAssembly, in the browser.
- The installer runs in the camera's Linux, started by an SD script. On the official firmware, only Linux can write the RTOS code region. On the optional [fast-boot firmware](fastboot.md), the installer runs in the RTOS.
- The hooks, the core, and the modules run in the camera's RTOS.

The core is per target. It owns the firmware hooks, the state of each capability, and the hot paths. A module is the same file on each camera. It gives data, such as settings, pages, and text, and callbacks that the core calls. A module reaches the firmware only through the core.

| Path | Contents |
| --- | --- |
| `crates/grkit-core` | Formats, effect compilation, artwork, runtime templates, package assembly. No file or process access. |
| `crates/grkit-build` | Firmware checks, patch generation, Clang and LLD invocation. |
| `crates/grkit-cli` | The `grkit` command. It embeds `runtime/` and `resources/`, so it works outside the checkout. |
| `crates/grkit-wasm` | Package assembly in the browser, with the same checks as the CLI. |
| `runtime/` | Native C and assembly: the installer, the hooks, and the core. `runtime/sdk/` is the module SDK. |
| `modules/` | The official modules. |
| `effects/` | Built-in effect definitions. See [effects/README.md](../effects/README.md). |
| `resources/` | Installation guides. |
| `web/` | The site: package builder and effect preview. See [web/README.md](../web/README.md). |

## Targets

A target is one firmware build. It gives the hook sites and the firmware addresses of that build. The targets of one camera line, such as the GR IV builds, form a family. The family lists its hooks once and shares the runtime code. Each build of a family must have each hook, each definition, and each feature of the family. So a change for one build fails until it reaches each build.

| Target | Cameras |
| --- | --- |
| `gr3-2.10` | GR III 2.10 and 2.20, GR IIIx 1.60 and 1.70, and their HDF models |
| `gr4-1.11` | GR IV 1.11, GR IV HDF, GR IV Monochrome |
| `gr4-1.20` | GR IV 1.20 and GR IV HDF 1.20, GR IV Monochrome 1.10 |
| `gr4-1.20-0528` | GR IV 1.20, the factory build of GR IV 30th Anniversary Edition cameras |

A build whose code moved only a little runs the runtime of another build through [relocation](#relocation). An HDF camera runs the firmware of its base camera. `camera.rs` lists the releases of each camera and the runtime of each release.

## Installation

At each power-on, a loader script in the RTOS waits until the camera mounts the card. Then it runs two steps on Linux and stays resident:

1. SCAN lists the modules and the effects and writes an allocation plan. A new package also copies its effects into the effects library. The loader allocates the arrays of the plan in the RTOS.
2. INSTALL links the modules, writes the effect records and the icons, restores the saved settings, and installs the hooks last.

The resident loader then saves the settings when they change.

INSTALL changes nothing on a firmware that it does not expect. Each address must resolve in the running image, and each hook site must hold its stock instruction. Each hook belongs to one extension point, and INSTALL installs only the points that the loaded modules need. When a camera has several targets, its package holds the installer of each, and the installer of the running build runs.

On the optional [fast-boot firmware](fastboot.md), the loader runs SCAN and INSTALL on the RTOS side right after power-on, through the fast-boot files of the package. It falls back to Linux if that SCAN writes no plan.

The loader writes `INSTALL.ERR` from the RTOS before each step and removes it after the install. So a start that stops leaves its step on the card. The scripts also write a boot log, `GRKIT/BOOT.TXT`, with the time of each step.

### Card layout

A package holds only `SCRIPT`. `GRKIT` at the card root is the data folder: no package holds it, so a new package, copied by replace or by merge, keeps it.

| Path | Contents |
| --- | --- |
| `SCRIPT/GRKIT` | the loader, the installers, the package list, and the modules, effects, and recipe library of the package |
| `GRKIT/EFFECTS` | the effects library |
| `GRKIT/MODULES` | the modules that a person adds |
| `GRKIT/STATE.TXT` | the saved settings |
| `GRKIT/RECIPES.TXT` | the recipes |
| `GRKIT/REPORT.TXT`, `GRKIT/BOOT.TXT` | the result and the boot log of the last starts |
| `GRKIT/SAFE` | starts the safe mode |

A package module loads only if the package list names it, and a module in `GRKIT/MODULES` replaces the package module with the same name. A new package copies its effects into the library once, so an effect that a person deleted stays deleted. INSTALL adds the recipes of a package library to `GRKIT/RECIPES.TXT` once for each library, so a recipe that a person deleted stays deleted. The installer never renames or deletes a file in the data folder, because the camera's file systems do not support it reliably.

### Report and safe mode

`GRKIT/REPORT.TXT` gives the GRKit version and target, the result of each module, and the installed effects.

While `GRKIT/SAFE` exists, no module loads, and only module management stays in MENU, so a person can turn off the module that stopped the camera. The safe mode does not start by itself, because a count of unfinished starts also counts each quick power-off.

## Modules

Each feature is a module, `<ID>.GRM`, linked at power-on. The same file runs on each camera. A module that does not link is skipped, and the others load.

| Module | Contents |
| --- | --- |
| `FILMLOOK` | a page of `CORE.TONE` |
| `FRAMING` | a page of `CORE.ASPECT`, the mask, and the ratio label |
| `GUIDES` | the guides |
| `DATE` | the date imprint |
| `GLOW` | independent Bloom and Halation around the highlights of each recorded picture |
| `GRAIN` | a page of `CORE.GRAIN` and `CORE.GRAIN_SIZE` |
| `RECIPES` | a page of `CORE.RECIPE` ([Recipes](#recipes)) |

The core keeps the value of each setting. The capabilities, `CORE.<KEY>`, are settings of the core. The core keeps each in a header word that its hooks read, so module code never runs in a hot path. INSTALL applies the patches of a capability only when a loaded module uses it.

The core calls module code for these parts:

- Pages give the rows in ADJ and MENU.
- Layers paint on the live view.
- Plane filters change each recorded picture.
- Events, timers, and UI callbacks run in the App task.

[Writing a module](modules.md) is the guide for module authors, and [the module reference](module-reference.md) gives the keys, services, and limits.

## Menu

GRKit adds an entry to the ADJ toolbar that lists the module pages. It reuses the native ADJ list, so it behaves like a native entry. Fn on a row opens the native settings page of the module.

### GRKit section of MENU

The Customize tab of MENU gets a GRKit section. It has **Enable GRKit**, a row for each module file, which turns the module on or off and opens its settings, **ADJ Order**, and **About GRKit**. This switch, which ADJ shows too, turns all modules off and on again and keeps the Enable of each. The settings pages are native pages of the camera. A module that is off loads stopped: the core calls none of its code, so on applies at once. A page with options can turn each of its values, or each effect, on or off in ADJ.

The menu text follows the camera language.

## Pictures

- **Film Look** (`CORE.TONE`) replaces the color matrix and tone curves of Standard for still pictures and the live view. It can also drive the camera's hue-sector stage. Each effect has its own Fn adjustments, kept apart from the native ones. The live-view highlight warning and histogram follow the white of the effect.
- **Framing** (`CORE.ASPECT`) selects a ratio and sets the native aspect to 3:2. `CORE.ASPECT_MODE` selects Crop or Guides Only, with Crop as the default. Crop changes JPEG dimensions. Guides Only marks composition on the 3:2 live view without applying the selected crop. Off and changes to the mode preserve the current native aspect. A later change to another native aspect hides the composition frame.
- **RAW default crop**: when Framing crops a shot, DNG defaults to native 3:2 and preserves the shot's digital crop setting. The DNG writer changes only `DefaultCropOrigin` and `DefaultCropSize`. The capture snapshot identifies the shot's Framing crop. Embedded previews keep the selected composition.
- **Guides** follow the framing area in both Framing modes, so that they divide the frame that the photographer composes in. Without a framing area, they follow the native aspect ratio.
- **Recorded pictures**: a hook on the JPEG encode changes the main picture, the screennail, and the thumbnail. It applies the selective color of the effect and the plane filters of the modules. Each shot keeps its own settings, so continuous shooting is correct. Grain uses the camera's grain stage, so the live view and DNG files have no grain.

## Recipes

A recipe is a named set of picture settings that OK applies in one step. It holds the look, Grain, Glow, white balance, the correction and noise reduction settings, and the exposure settings: the compensation, the Auto ISO upper limit, metering, and the ND filter. The look is a Film Look effect with its Fn adjustments, or a native Image Control type with its parameters. The capability is `CORE.RECIPE`, and the `RECIPES` module is its page in ADJ.

- **Store.** The camera holds one list of at most 64 recipes in `GRKIT/RECIPES.TXT`. The loader saves it like the state file, but with its own change count, so a change of a setting does not write the recipes again. On a connected card, the site reads the file and writes it again in full. A ZIP package carries a library in `SCRIPT/GRKIT/RECIPES.BIN`. INSTALL adds each of its recipes of the camera line that the list does not hold yet, and keeps the hash of the library, so the next start adds nothing. The undo steps are only in memory, and a restart clears them.
- **On and off.** At most 40 recipes are on, and they come first in the list. The camera lists only them in ADJ and MENU. It keeps the others in the file for the site, which turns recipes on and off. A save on the camera adds a recipe that is on, after the others that are on. INSTALL keeps this order and turns off each recipe that is on past 40.
- **Site.** The camera makes and edits recipes. The site does not change the settings of a recipe: it turns recipes on and off and deletes them, but not a preset. It lists the presets (`grkit_core::recipes::presets`) after the recipes, off, and a preset joins the list when the person turns it on. A recipe that the list holds takes the on or off of a new package library. A package library holds every recipe, so the site gives back the whole list from a GRKit.zip. Each preset has a picture that the camera recorded with the recipe, because the preview tables end at diffuse white and cannot show the lights that Glow needs (`web/scripts/build-landing.mjs`).
- **Record.** Each recipe is the same five words in the library, in the recipe file, and in the core. The words hold portable values, so a recipe of one camera line fails the checks of another. An effect recipe holds the low 15 bits of the FNV-1a hash of the effect ID, and the core hashes the ID of each record to find the effect. The ImageTone code of a record does not serve, because INSTALL moves the code of an effect whose hash another effect has. `grkit_core::recipes` defines the words and the library file, and the site uses them through the wasm build.
- **Native settings.** Apply writes each native setting by its name and value label through the `Set` command of the RTOS shell command `UserData`. A read takes the fields of the settings record. The build reads the names, the labels, and the field offsets of each build from its firmware (`grkit_build::userdata`) and generates the tables of the core. A GR IV Monochrome type has no label, so apply calls the type setter with its stored value.
- **Module settings.** A recipe holds Glow while the Glow module is loaded. The core finds each Glow setting in the settings table by its state file ID. Apply sets each one that the recipe holds. Without the module, apply leaves Glow out and says so, unless the recipe's Glow is Off.
- **Exposure settings.** Each one is a single setting that all shooting modes share. The compensation and the ISO limits are floats, so apply writes the number as text and a read rounds the field to sixths or thirds of a stop with integer code. Apply rounds the compensation to the exposure step of the camera and the ISO limit to its ISO step, and keeps the upper limit at or above the lower one. A save on the camera holds no exposure settings, because they belong to one scene, and they do not decide which recipe is the current one. An undo step holds them.
- **Keys.** Each word of a recipe or a name has its own range of keys in the recipe file, so a later word adds a range and the keys of a saved file stay valid (`grkit_core::runtime::recipe_key`). A recipe that a person keeps out of ADJ has a flag in its first word, and a recipe that is off has a flag in its fifth word.
- **Checks.** Apply first checks each value against the tables of the running build. A value that the camera does not have changes nothing. The settings before an apply become an undo step. Without the Grain module, apply leaves out a Grain that is not Off and says so. A row whose effect is not installed shows a mark after the name, and REPORT.TXT names each recipe that does not apply in full.
- **MENU.** The page of the module in MENU has the form of the native User Mode page. Options chooses the recipes that ADJ shows, as the Options of other modules choose their values. Apply, Rename, and Delete each open a list of the recipes that are on, and Delete asks with a confirm box. Save Current Settings adds a recipe. The undo steps stay in ADJ.
- **Names.** A person types the name of a recipe on the camera: after Save, or with Rename from Fn on the recipe or from its page in MENU. The name is at most 16 printable ASCII characters. The recipe file keeps it in four words. A recipe without a name shows an automatic name: its look and a number. GRKit borrows the camera's own keyboard, the one of the copyright holder. While GRKit holds the keyboard, three hooks give the keyboard the GRKit text, take the text on OK instead of the copyright setter, and limit its length. The copyright holder setting never sees the text. A change to another controller ends the borrow.

## Relocation

Some cameras show a supported version but run another build of it. For each firmware address, the runtime stores a window of the build it was made from. INSTALL finds each window in the running image and moves each reference. The search takes seconds on a camera, so INSTALL first checks the addresses that an earlier search found, kept in `GRKIT/RELOC.BIN`. A window that is not found, or found twice, stops INSTALL with status 47 before it changes the firmware.

## Porting

If the code of a new firmware version moved only a little, the existing target runs on it: add the release to `camera.rs`. Otherwise, the version needs a new target in the family of the existing one. `grkit port` drafts it: it finds each address of the existing target in the new build. It cannot find an address whose code changed, and it does not check values that are not addresses.

1. Run `grkit port --target <target> --firmware <its firmware> --to <new firmware> --out <directory>`. If the output gives `"relocatable": true`, add the release to the existing target instead.
2. Make the build header and the target module from the drafts. Resolve each `PORT` comment, and check each value that is not an address.
3. Register the target in `crates/grkit-build/src/target.rs`, `runtime/firmware.h`, and `camera.rs`, then run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`.
4. Add the firmware zip to the private firmware backup (`docs/releasing.md`).
5. Run `grkit port` again with `--check <new target>`, then the tests.

## Code areas

Private code uses the existing executable mapping after the RTOS image, below `0x55000000`. GRKit changes no other image bytes except hook sites and the listed instruction patches.

The core components share this area without fixed partitions. The builder places Processing, Menu, and Framing in order, using each component's compiled size. Each component starts on a 256-byte boundary. The first 256 bytes after the tail base remain unused.

```text
RTOS image | page alignment | 256-byte gap | Processing | Menu | Framing | modules | free space
                           ^ tail base                  (each start is aligned)             ^ 0x55000000
```

The aligned end of Framing determines the core span, `TAIL_BYTES`. Modules start immediately after that span. A larger component moves the following components and modules, without changing the module ABI or the executable mapping.

At installation, relocation finds the running image's end and aligns the tail base to 4 KiB. The installer checks that the core and selected modules fit before writing their contents. It publishes code before enabling firmware hooks and keeps the existing cache synchronization.

The builder rejects address overflow, overlapping patches, and changed hook instructions. `build.json` reports each component's address and size. Its `code_memory` section reports the core span and remaining executable capacity before modules load.

The writable script tail, `GRKIT_TAIL_SIZE`, is separate. It holds image scratch buffers, text, and Toning curves in heap memory. Code growth does not change that allocation.

## Testing

- Rust tests pin the formats, the artwork, the generated headers, and the loader scripts.
- Host C tests compile parts of the runtime with a host compiler.
- `grkit module test` runs the host test of a module against a fake core.
- `--ignored` tests build the runtime with an ARM toolchain. See [CONTRIBUTING.md](../CONTRIBUTING.md).
