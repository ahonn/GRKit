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

The HDF model of a camera runs the same file. GR IV Monochrome 1.10 has the official file of GR IV 1.20, so the same fast-boot firmware runs on it. The factory build of GR IV 30th Anniversary Edition cameras has no fast-boot firmware.

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

A GRKit package has two files for each release of its camera: the fast-boot file `SCRIPT/GRKIT/F<project><version>.BIN` and its install file `SCRIPT/GRKIT/I<project><version>.BIN`. The same package works on the official firmware.

1. The loader script probes the firmware.
2. On the fast-boot firmware, it reads the first 1 KB of the fast-boot file of the release and calls it. That head loads the whole file to the top of the RTOS code region and runs SCAN. SCAN keeps the whole plan in the memory of the file and gives the plan header to the loader in the script array, so it writes no plan file to the card. The file reads each card file and directory of the start once and keeps it in memory until INSTALL ends. The listing of a folder gives the size and the time stamp of each file, so a file of a listed folder needs no status call. `GRKIT/IMPORTED.TXT` also records the size and the time stamp of each package effect, so a start reads a package effect only after it changed. After a successful INSTALL, the file writes the effects of the library and the modules that the start read into one file, `GRKIT/CACHE.PAK`, with the size and the time stamp of each. The next start reads that file in one call and takes a file from it only while its listing gives the same size and time stamp. A sum covers the whole file, so a file that a power loss cut is not used. GRKit packages give each file a time that follows from its bytes, so a copy of a package with other bytes does not look unchanged.
3. The loader allocates the arrays of the plan and calls the install file. The install file is the same head with the install vector and a sum for each 4 KB code page. It does not load the file again: it compares each code page in memory with its sum, and calls INSTALL in the loaded file only if no page changed. INSTALL first compares the data of the file with the sums that the last call left, then reads the plan from memory. INSTALL gives its status in the script array and writes no `INSTALL.OK` or `INSTALL.LCK`. After a not-ready status, the loader calls the install file again, and INSTALL starts as at the first call.
4. If SCAN writes no plan, or INSTALL fails or does not run, the loader runs SCAN and INSTALL on Linux, as on the official firmware. After a failed INSTALL, the new plan must have the sizes of the arrays that the loader already allocated. A failed rollback (status 24) stops the loader instead.

When a start does not use the fast-boot files, or stops using them, the loader adds one line to `GRKIT/BOOT.TXT`. A start that installs through them adds no line.

| Line | Meaning |
|---|---|
| `fast-boot off: official firmware` | The probe found no fast-boot firmware. |
| `fast-boot off: interface N` | The fast-boot firmware has another interface version. |
| `fast-boot RELEASE off: not in this package` | The package has no fast-boot file for the firmware release. |
| `fast-boot RELEASE off: files missing` | The fast-boot file or its install file is not on the card. |
| `fast-boot RELEASE scan STATUS, Linux used` | SCAN wrote no plan. |
| `fast-boot RELEASE install STATUS, Linux used` | INSTALL failed. |
| `fast-boot RELEASE install STATUS at ADDRESS count N, Linux used` | Other code wrote into the memory of the fast-boot file after SCAN, so INSTALL did not run. |

STATUS is an installer status (`runtime/install_status.h`), 998 when the head did not run SCAN or INSTALL (the file is not complete), or 999 when the firmware did not read the file. 997 means that N code pages changed, and 996 that N data blocks of 64 KB changed. ADDRESS is the address of the first changed page or block, in decimal.

The code of GRKit ends before the fast-boot file, which runs from 0x54c00000 (`runtime::FAST_BASE`).

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
