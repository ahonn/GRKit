# GRKit site

The site lets a user select a camera, the modules, and the effects, preview the effects, and download `GRKit.zip` or write it to an SD card. The browser builds the package with `grkit-wasm`. The only server code is the Cloudflare Pages worker of the root page, which sends a visitor to the page of their language.

The site serves these files:

| Path | Source |
|---|---|
| `index.html`, `zh-hans/`, `zh-hant/`, `ja/`, `ko/`, the `fastboot/` page of each, `404.html`, `sitemap.xml`, `robots.txt`, `_worker.js`, `_routes.json`, `assets/`, `_headers`, `THIRD_PARTY_LICENSES.txt` | `pnpm build` in this directory |
| `runtime/<target>/` | `grkit site`, for each runtime build of `grkit release-plan` |
| `effects/<target>/` | `grkit effects publish`, for each runtime build of `grkit release-plan` |

Each language has its own page. `scripts/prerender.mjs` renders the first screen of each page into its HTML, with the canonical and hreflang links, so search engines read the text without the script. The worker (`src/edge.ts`) runs only for `/`. It sends the visitor to the language that they chose before, else to the browser language. The country of the IP address decides only when the browser sends no language.

`THIRD_PARTY_LICENSES.txt` gives the licenses of the bundled JavaScript packages and fonts, and of the Rust crates in the WebAssembly module. `pnpm build` writes it with `scripts/third-party-licenses.mjs`. The script reads the JavaScript licenses from the Vite build and the crate licenses from `cargo metadata`, so Cargo must be on `PATH`.

The site serves its fonts and scripts itself and sends no request to another host.

## Develop

The tools are Node.js 24, pnpm, the `wasm32-unknown-unknown` Rust target, and wasm-bindgen 0.2.128. The wasm-bindgen version must equal the version that `Cargo.toml` pins.

1. Install the dependencies: `pnpm install`.
2. Build the browser bindings: `pnpm wasm`. If `wasm-bindgen` is not on PATH, set `WASM_BINDGEN` to its path.
3. Make the site data from the repository root. The commands need the official firmware and the ARM toolchain. See [CONTRIBUTING.md](../CONTRIBUTING.md).

   ```sh
   grkit build --target gr3-2.10 --firmware fwdc239b.bin --out dist/runtime-local
   grkit site --build dist/runtime-local --out dist/site-data
   grkit effects publish --source effects --firmware fwdc239b.bin --out dist/site-data/effects/gr3-2.10
   sh .github/scripts/install-guide-tools.sh dist/guide-tools
   sh .github/scripts/build-guides.sh grkit dist/guide-tools dist/site-data/guides
   ```

   For GR IV, also build `gr4-1.11` from `fwdc248b.bin`, add it with `grkit site`, and publish its effects with `--target gr4-1.11`. Without them, only the GR III and GR IIIx packages work.

4. Start the development server: `pnpm dev`. The server reads the site data from `../dist/site-data`. To use another directory, set `GRKIT_SITE_DATA`.

To add Plausible analytics to a build, set `PLAUSIBLE_HOST` to the Plausible origin, for example `https://plausible.example.com`. The build then adds the script to `index.html` and allows it in the Content-Security-Policy of `_headers`. Release reads the repository variable `PLAUSIBLE_HOST`.

The site data must come from the current source. If the runtime templates change, the browser rejects an older runtime with "Runtime scripts differ from the production templates".

## Check

```sh
pnpm typecheck
pnpm test
pnpm build
```

## Preview

The page opens with a GR IIIx sample photo. A user can also select a DNG from any camera. A Worker decodes it with LibRaw into linear sRGB in the browser, and a second Worker applies the preview tables of `grkit effects publish`. Photos are not uploaded to a server.

The effects are fitted to the GR III, and LibRaw's color processing differs from the camera's. The previews are therefore approximations.

LibRaw is distributed under CDDL-1.0. `THIRD_PARTY_LICENSES.txt` includes its notices and a link to its source.

## Fast start

The Fast start page (`/fastboot/` under each language page) makes the fast-boot firmware (`docs/fastboot.md`) from an official firmware update file that the user selects. `patchFirmware` of `grkit-wasm` refuses any other file and checks the result against its SHA-256. The file is not uploaded.

## LUT import

The Import a LUT row at the end of the effect list, and the link under the selection buttons, open a dialog. The dialog fits a `.cube` file or a LUT image into an effect. It uses the fit of `grkit effects import` ([Import a LUT](../effects/README.md#import-a-lut)). The page fits over the `base_gamma` of the `gr3-2.10` catalog. A catalog without it hides the row and the link.

1. The user selects a file. The dialog fills in the effect name and the icon from the file name.
2. The Auto input is a guess from the LUT data. The dialog says what the guess cannot tell, so the user can select the input.
3. After the fit, the dialog shows the input, how closely the effect follows the LUT, and the warnings.
4. The page adds the effect to the table and selects it if the grade is A or B and the package has room.

The page checks every catalog effect until the user changes the selection. A user can import any number of LUTs, but a package holds at most 20 effects, so at most 20 can be checked. The browser keeps the imported effects in IndexedDB (`src/import/store.ts`), so a later visit has them. The store is a convenience of one browser: without it, the page works as before. For an editor preset, the dialog offers an identity HALD image. The user applies the preset to it and imports the export with Standard input.

## SD card

The first section of the page selects how to install: write to an SD card in a card reader, through the File System Access API, or download `GRKit.zip`. The section shows only in a browser that can write to a card, Chrome and Edge on a computer, where the card mode is the default. In other browsers, the page has only the download. The page remembers the choice in `localStorage`.

1. The page reads the card. The first line of `GRKIT/REPORT.TXT` gives the version and selects the camera. The modules of `PACKAGE.TXT` and the effects of the library become the selection. An effect file that the catalog does not have shows as an extra row.
2. The user changes the selection. Badges show what a write adds or removes.
3. The last section lists the changes. Nothing on the card changes until the user confirms them.

A write (`applyChanges` in `src/card.ts`):

- It writes the `SD_ROOT` files of the verified package (`Package.cardFiles`), only the ones that differ, as a merge copy of `SD_ROOT` does. It keeps `GRKIT` and the `SCRIPT/PLUGINS` folder of GRKit 1.2, which the first power-on migrates. The ModeSet files go only on a card without GRKit.
- It writes `PACKAGE.TXT` and `SCRIPT/STARTUP.TTL` last, reads each file back, and deletes the `._` files that macOS adds for the files that it wrote.
- It adds and removes effects in the library `GRKIT/EFFECTS` when the library exists. SCAN migrates the effects of GRKit 1.2 only when it creates the library, so the page does not create it, and the effects of GRKit 1.2 cannot be removed before that power-on.
- It deletes the package files that the new `PACKAGE.TXT` does not name. It never writes to `GRKIT/MODULES`, because a file there replaces a newer package module.

In card mode, Uninstall GRKit acts on the connected card. It shows only while the card has GRKit, or has the files that turn SD scripts off:

- Uninstall (`removeGrkit`) deletes `SCRIPT/GRKIT`, `SCRIPT/PLUGINS`, and `SCRIPT/STARTUP.TTL` when it runs one of them. Other files in `SCRIPT` stay. The data folder `GRKIT` goes only when the user also selects the settings and effects. A dialog asks for a confirmation first, with the choices to delete the settings and effects too and to turn SD scripts off.
- Turn off SD scripts writes the ModeSet file with the disable command and `DEVELOP.MOD`. When the card has that ModeSet file, the page offers to delete the two files after the camera step.

`src/card.test.ts` checks this logic against an in-memory card that behaves like a FAT card on macOS.

## Default sample

The default image is Sakiko buys a delicious soup at Kitchen Grizzly by MIKI Yoshihito, a GR IIIx JPEG under CC BY 2.0. `public/samples/kitchen-grizzly.json` records its source, license, and hashes. The site includes the attribution in `THIRD_PARTY_LICENSES.txt`, and the landing page credits the photo with a link to the source where it first shows.

`public/samples/kitchen-grizzly.grlinear` holds linear pixels that `scripts/build-sample.mjs` estimates from the JPEG. To make it again:

1. Download the JPEG from the source URL in `public/samples/kitchen-grizzly.json` to a directory outside the repository.
2. Get `effects/gr3-2.10/preview/standard.lut` from the site data, for example from the site archive of a release.
3. From `web/`, run `node scripts/build-sample.mjs '/path/to/Sakiko_buys_a_delicious_soup_at_Kitchen_Grizzly._(53311001084).jpg' /path/to/standard.lut`. The script checks the SHA-256 of the JPEG first.
4. Run `pnpm test`. The sample test checks the hash, the attribution, the dimensions, and the pixel format.
