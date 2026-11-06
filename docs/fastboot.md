# Fast-boot firmware

GRKit does not need this firmware. On the official firmware, GRKit loads from the SD card after the camera's Linux mounts the card, about 8 to 10 seconds after power-on.
The fast-boot firmware is the official firmware with three small changes. With it, SD scripts can run code from the card at power-on, and GRKit loads before Linux mounts the card.

> [!WARNING]
> The fast-boot firmware has been tested on a GR III camera only.
> To install it, you flash the camera with a firmware file that Ricoh did not make. If an update fails, the camera may not start.

## Changes

The changes are in the RTOS of the official update file. The rest of the firmware does not change. The file gets new check words and sizes, and the changed parts of the RTOS are stored uncompressed.

1. The sections of the RTOS code region (0x53000000 to 0x55000000) become writable.
2. `SYS_buildinfo` gets the probe of `fastboot/patch/probe.S`.
3. `SYS_memovr` gets the loader of `fastboot/patch/stub.S`.

The official firmware has the two commands, but uses neither. Each release table in `fastboot/releases/` gives the SHA-256 of the official update file, of its RTOS, and of the result.

| Release | Official update file |
| --- | --- |
| GR III 2.10, 2.20 | `fwdc239b.bin` |
| GR IIIx 1.60, 1.70 | `fwdc243b.bin` |
| GR IV 1.11, 1.20 | `fwdc248b.bin` |

The HDF model of a camera runs the same file. GR IV Monochrome and the factory build of GR IV 30th Anniversary Edition cameras have no fast-boot firmware.

## Script interface

Any SD script can use the two commands. A script gives a script array by its number in the order of `intdim`, where 0 is the first array.

`SYS_buildinfo INDEX` writes four words into array `INDEX`, if the array has at least four words:

| Word | Value |
| --- | --- |
| 0 | `1129202771` (`0x434e4453`) |
| 1 | the interface version, 1 |
| 2 | the project number: 588 GR III, 609 GR IIIx, 636 GR IV |
| 3 | the firmware version: 210 for 2.10 |

The official command prints the build information and does not change the array.

`SYS_memovr PATH INDEX [call]` reads the card file `PATH` into array `INDEX`. `PATH` is an RTOS path with 8.3 names, such as `C:\SCRIPT\APP\CODE.BIN`.

- The command reads at most the size of the array.
- After the read, it cleans the data cache and invalidates the instruction cache.
- With a fourth argument, it calls the array as ARM code, with r0 = the address of the array and r1 = the bytes read. The code runs in the script task and returns to the script.
- It does nothing if the index is not that of an array, or if the file is empty, missing, or not read.

> [!CAUTION]
> On the official firmware, `SYS_memovr` is a memory test that overruns a buffer.
> Send `SYS_memovr` only after `SYS_buildinfo` gave word 0 = `1129202771` and word 1 = 1.

```text
intdim probe 256
sendln 'SYS_buildinfo 0'
if probe[0] <> 1129202771 goto official
if probe[1] <> 1 goto official
sendln 'SYS_memovr C:\SCRIPT\APP\CODE.BIN 0 call'
:official
```

A later interface has another version in word 1. A script that does not know the version must not use the commands.

## How GRKit uses it

A GRKit package has a fast-boot file for each release of its camera, `SCRIPT/GRKIT/F<project><version>.BIN`, and `SCRIPT/GRKIT/FRUN.BIN`. The same package works on the official firmware.

1. The loader script probes the firmware.
2. On the fast-boot firmware, it reads the first 1 KB of the fast-boot file of the release and calls it. That head loads the whole file to the top of the RTOS code region and runs SCAN.
3. The loader allocates the arrays of the plan and calls `FRUN.BIN`, which runs INSTALL.
4. If SCAN writes no plan, the loader runs SCAN and INSTALL on Linux, as on the official firmware.

After a fast SCAN writes a plan, INSTALL runs only from `FRUN.BIN`. If that INSTALL fails, the loader reports the error and does not try the Linux path. If `FRUN.BIN` is missing, the loader waits for it as for `INSTALL.SH`. The code of GRKit ends before the fast-boot file, which runs from 0x54c00000 (`runtime::FAST_BASE`).

`runtime/fastboot/` has the RTOS side of the installer and the head. `runtime::fastboot` in `grkit-core` names the files.

## Make the fast-boot firmware

A person who does not build GRKit uses the Fast start page of grkit.app (`/fastboot/`), which runs the same code in the browser, and the [fast start guide](../resources/FASTBOOT.md). The page takes the official ZIP from Ricoh and makes the latest release of each camera only (`LATEST` in `web/src/fastboot.ts`). The `grkit` command takes the official ZIP or its update file of any release in the table of releases.

1. Download the official firmware of your camera and version from Ricoh.
2. Run `grkit firmware patch` with the official ZIP or the update file in it:

   ```sh
   grkit firmware patch --firmware gr3_v220.zip --out patched/fwdc239b.bin
   ```

   The command refuses any file that is not the official ZIP or update file of a release in the table. It checks the result against the SHA-256 of the release.
   Expected result: JSON output with the release, such as `gr3-2.10`, and the SHA-256 of the new file.

3. Install the new file as an official update, with the procedure of the camera's manual.

To go back to the official firmware, install the official update file of the same version or of a later version in the same way.

## Add a release

1. Add `fastboot/releases/<camera>-<firmware>.json` with the hashes, the addresses of the functions that the handlers call, and the assembled handler code.
2. Add `runtime/fastboot/targets/<camera>-<firmware>.h` with the RTOS calls of the installer.
3. Run the tests. `fastboot_code_matches_its_source` assembles `fastboot/patch/` with the addresses of each table and compares the result with its code. `fastboot_official_files_patch_to_their_result` patches the official files in `GRKIT_TEST_FASTBOOT_FIRMWARE`.
4. If a table has no result yet, run `grkit firmware patch` with the official file. The error gives the size and SHA-256 of the new file for `result`.
