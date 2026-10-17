# GRKit site

The site lets a user select a camera, the modules, and the effects, preview the effects, and download `GRKit.zip`. The browser builds the package with `grkit-wasm`. No server code runs.

The site serves these files:

| Path | Source |
|---|---|
| `index.html`, `assets/`, `_headers` | `pnpm build` in this directory |
| `runtime/gr3-2.10/`, `runtime/gr4-1.11/` | `grkit site` |
| `effects/gr3-2.10/`, `effects/gr4-1.11/` | `grkit effects publish` |

## Develop

The tools are Node.js 24, pnpm, the `wasm32-unknown-unknown` Rust target, and wasm-bindgen 0.2.128. The wasm-bindgen version must equal the version that `Cargo.toml` pins.

1. Install the dependencies: `pnpm install`.
2. Build the browser bindings: `pnpm wasm`. If `wasm-bindgen` is not on PATH, set `WASM_BINDGEN` to its path.
3. Make the site data from the repository root. The commands need the official firmware and the ARM toolchain. See [CONTRIBUTING.md](../CONTRIBUTING.md).

   ```sh
   grkit build --target gr3-2.10 --firmware fwdc239b.bin --out dist/runtime-local
   grkit site --build dist/runtime-local --out dist/site-data
   grkit effects publish --source effects --firmware fwdc239b.bin --out dist/site-data/effects/gr3-2.10
   ```

   For GR IV, also build `gr4-1.11` from `fwdc248b.bin`, add it with `grkit site`, and publish its effects with `--target gr4-1.11`. Without them, only the GR III and GR IIIx packages work.

4. Start the development server: `pnpm dev`. The server reads the site data from `../dist/site-data`. To use another directory, set `GRKIT_SITE_DATA`.

The site data must come from the current source. If the runtime templates change, the browser rejects an older runtime with "Runtime scripts differ from the production templates".

## Check

```sh
pnpm typecheck
pnpm test
pnpm build
```

## Sample photos

The preview has a generated color chart. To add a sample photo:

1. Put a Standard JPEG from the camera in `public/samples/`.
2. Add it to `SAMPLES` in `src/preview/samples.ts` with `photoSample()`, with a label in each language.

The preview assumes Standard input. A photo with another image setting, or from another camera, shows the effect less accurately.
