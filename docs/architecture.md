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
- The installer runs in the camera's Linux, started by an SD script. Only Linux can write the RTOS code region.
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

The loader writes `INSTALL.ERR` from the RTOS before each step and removes it after the install. So a start that stops leaves its step on the card. The scripts also write a boot log, `GRKIT/BOOT.TXT`, with the time of each step.

### Card layout

A package holds only `SCRIPT`. `GRKIT` at the card root is the data folder: no package holds it, so a new package, copied by replace or by merge, keeps it.

| Path | Contents |
| --- | --- |
| `SCRIPT/GRKIT` | the loader, the installers, the package list, and the modules and effects of the package |
| `GRKIT/EFFECTS` | the effects library |
| `GRKIT/MODULES` | the modules that a person adds |
| `GRKIT/STATE.TXT` | the saved settings |
| `GRKIT/REPORT.TXT`, `GRKIT/BOOT.TXT` | the result and the boot log of the last starts |
| `GRKIT/SAFE` | starts the safe mode |

A package module loads only if the package list names it, and a module in `GRKIT/MODULES` replaces the package module with the same name. A new package copies its effects into the library once, so an effect that a person deleted stays deleted. The installer never renames or deletes a file in the data folder, because the camera's file systems do not support it reliably.

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
| `GRAIN` | a page of `CORE.GRAIN` and `CORE.GRAIN_SIZE` |

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

The Customize tab of MENU gets a GRKit section. It has a row for each module file, which turns the module on or off and opens its settings, **ADJ Order**, and **About GRKit**. The settings pages are native pages of the camera. A module that is off loads stopped: the core calls none of its code, so on applies at once. A page with options can turn each of its values, or each effect, on or off in ADJ.

The menu text follows the camera language.

## Pictures

- **Film Look** (`CORE.TONE`) replaces the color matrix and tone curves of Standard for still pictures and the live view. It can also drive the camera's hue-sector stage. Each effect has its own Fn adjustments, kept apart from the native ones. The live-view highlight warning and histogram follow the white of the effect.
- **Framing** (`CORE.ASPECT`) crops still pictures to a ratio. The live view stays 3:2 with a mask, and DNG files keep the full sensor data.
- **Recorded pictures**: a hook on the JPEG encode changes the main picture, the screennail, and the thumbnail. It applies the selective color of the effect and the plane filters of the modules. Each shot keeps its own settings, so continuous shooting is correct. Grain uses the camera's grain stage, so the live view and DNG files have no grain.

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

All private code is in the reservation after the RTOS image. GRKit changes no byte of the image except the hook sites and a few in-place patches. The builder rejects an overflow, a changed hook instruction, and overlapping patches.

## Testing

- Rust tests pin the formats, the artwork, the generated headers, and the loader scripts.
- Host C tests compile parts of the runtime with a host compiler.
- `grkit module test` runs the host test of a module against a fake core.
- `--ignored` tests build the runtime with an ARM toolchain. See [CONTRIBUTING.md](../CONTRIBUTING.md).
