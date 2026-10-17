# Image Control effect sources

Each directory with `effect.json` defines one effect. The directory name is its stable effect ID.
Edit the name, signed Q13 matrix, RGB curves or curve knots, and icon to change an effect. Add a directory to add an effect.

The compiler writes self-contained GRFX0001 files with a 40 x 40 icon. CRC32 detects corruption. Neither the CRC nor the ID authenticates an author.
`golden.json` pins the compiled file and artwork hashes. Change it only for an intentional definition or artwork change.

## Definition fields

| Field | Required | Value |
|---|---|---|
| `schema` | Yes | `grkit-effect-v1` |
| `effect_id` | Yes | The stable ID, the same as the directory name |
| `name` | Yes | 1 to 16 printable ASCII characters |
| `flags` | Yes | 0, or 1 for monochrome |
| `matrix` | Yes | Nine signed Q13 coefficients, row-major. Each row sums to 8192. |
| `curves` or `curve_knots` | Yes | Three 256-sample curves of 0 to 16383, or knots for each channel |
| `icon` | Yes | `label` and `shape` (`film`, `filter`, or `plain`) |
| `attribution` | Yes | An object with the origin, source, and license |
| `post_matrix` | No | Nine signed Q13 coefficients, row-major, after the tone curves |

A monochrome effect cannot have `post_matrix`.

## Effect catalog

The site and the release packages publish every effect in this directory. Each effect is a fit of one film simulation LUT:

| Effect | ID | Target |
|---|---|---|
| Portra 400 | `portra-400-endura-v1` | Kodak Portra 400 on Portra Endura paper |
| Gold 200 | `gold-200-premier-v1` | Kodak Gold 200 on Endura Premier paper |
| Ektar 100 | `ektar-100-edge-v1` | Kodak Ektar 100 on Ektacolor Edge paper |
| Pro 400H | `pro-400h-crystal-v1` | Fujifilm Pro 400H on Crystal Archive Type II paper |
| X-Tra 400 | `xtra-400-crystal-v1` | Fujifilm X-Tra 400 on Crystal Archive Type II paper |
| Vision3 250D | `vision3-250d-2383-v1` | Kodak Vision3 250D on Vision 2383 print film |
| Vision3 500T | `vision3-500t-2393-v1` | Kodak Vision3 500T on Vision Premier 2393 print film |
| Portra 800 +2 | `portra-800-push2-v1` | Kodak Portra 800 pushed two stops, on Portra Endura paper |
| Kodachrome 64 | `kodachrome-64-v1` | Kodak Kodachrome 64 slide film, scanned |
| Velvia 100 | `velvia-100-v1` | Fujifilm Velvia 100 slide film, scanned |
| Portra 160 | `portra-160-ultra-v1` | Kodak Portra 160 on Ultra Endura paper |
| Tri-X 400 | `trix-400-mono-v1` | Kodak Tri-X 400 on Polymax Fine-Art paper, grade 2 |
| Tri-X 400 Red | `red-filter-mono-v1` | Tri-X 400 behind a Wratten 25 red filter, on Polymax Fine-Art paper |
| FP-100C | `fp-100c-v1` | Fujifilm FP-100C instant film |
| Aerocolor IV | `aerocolor-iv-v1` | Kodak Aerocolor IV 2460 aerial negative on Portra Endura paper |
| EXR 200T | `exr-200t-2383-v1` | Kodak EXR 200T 5293 on Vision 2383 print film |

Menu names are the names of the simulated films. They identify the materials, not an endorsement. [NOTICE.md](NOTICE.md) has the disclaimer.

The first eleven LUTs come from [spektrafilm](https://github.com/andreavolpato/spektrafilm) 0.3.4. The other five come from [Spectral Film LUT](https://github.com/JanLohse/spectral_film_lut). `sources/` holds the scripts that bake each LUT, the upstream notices, and `manifest.json`, which records the SHA-256 of each LUT. The LUTs themselves are not in the repository.

To refit an effect:

1. Install the generator at the revision in `sources/manifest.json`.
2. Run `python sources/bake_spektrafilm.py LUTS` or `python sources/bake_spectral_film_lut.py LUTS`.
3. Compare the SHA-256 of `LUTS/ID.cube` with the manifest. Floating-point results can differ on another platform.
4. Run `grkit effects import --input vlog` with the effect's ID, name, label, and shape.
5. Copy `license`, `target`, and `generator` from the old `attribution`.

The two monochrome fits have identical matrix rows and curves. Set `flags` to 1 for them.

Attribution and licenses are in [NOTICE.md](NOTICE.md). The installation guide in each package links to it.

## Processing domain

Effects run on the camera's look matrix before Standard base gamma. RGB curves index base-gamma output divided by 64.
They are not display-sRGB JPEG transforms. Imported fits assume 18% gray at base-gamma index 7.8.

The optional post-curve matrix (`post_matrix`) acts on gamma-encoded RGB after the tone curves. The runtime multiplies the YCbCr matrix by it.
Effects add no grain, glow, or hue-sector correction.

## Import a .cube preset

`grkit effects import` fits a 3D or 1D `.cube` LUT to the effect model and writes a new definition.
The fit needs the Standard base gamma. The command reads this table from your copy of the official GR III 2.10 firmware. The repository contains no firmware data.

```sh
grkit effects import --cube look.cube --firmware fwdc239b.bin --input srgb \
  --id my-look-v1 --name "My Look" --label FV --shape film --out effects
```

1. Set `--firmware` to the official GR III 2.10 firmware file.
2. Select `--input` for the encoding of the LUT input:
   - `srgb`: sRGB-encoded display RGB. The importer clips scene-linear light to 0..1 before the sRGB curve.
   - `linear`: scene-linear RGB with sRGB primaries.
   - `vlog`: Panasonic V-Log over V-Gamut primaries, as spektrafilm writes its LUTs.
3. Optional: add `--exposure EV` to change the scene-linear light before the LUT. The default is 0.
4. Optional: add `--no-post` to fit no post-curve matrix.
5. Run the command. The command writes `OUT/ID/effect.json` and prints a JSON summary.
6. Add the license of the LUT to `attribution`. Then run `grkit effects build`.

If `OUT/ID` exists, the command stops and writes nothing.
The command checks the ID, name, label, and shape before the fit. Labels have one to four capitals or digits.
The command checks the firmware SHA-256 and the shape of the base gamma table. If a check fails, the command stops.

The fit follows `cube_fit.py`, `joint_fit.py`, and `lab_fit.py` of gr3-film-mods (MIT):

- The model is `Curve_c(BaseGamma(K (M L)_c)) / 16384`, then the post matrix `P` on the gamma-encoded result.
- `BaseGamma` is the still-path Standard base gamma from the firmware. `K` puts 18% gray at base-gamma index 7.8. This index is an assumption.
- The samples are a gray ramp, the ColorChecker 24 from -2 to +2 EV, and 6000 seeded random colors.
- The importer alternates curve and matrix fits. Then it fits `P` on CIELAB residuals with a penalty that keeps neutrals neutral.
- The matrix and `P` are signed Q13 with rows that sum to 8192. The importer omits `post_matrix` when `P` is the identity.
- The YCbCr registers hold -256 to 255 in 1/256 units. A penalty keeps each coefficient of BT.601 x `P` at or below 240, so the hue rotation has room. If the quantized `P` exceeds 240, the importer moves `P` toward the identity until it fits.

The LUT output must be sRGB-encoded display RGB. The parser accepts `TITLE`, `LUT_3D_SIZE` (2 to 256), `LUT_1D_SIZE`, `DOMAIN_MIN`, `DOMAIN_MAX`, and `LUT_1D_INPUT_RANGE` or `LUT_3D_INPUT_RANGE`. It rejects a file with both a 1D and a 3D table.

`attribution.fit` records CIEDE2000 between the fit and the LUT on the ColorChecker at 0 EV. `mean_de` and `max_de` use the quantized fit. `standard_mean_de` uses the Standard look matrix with no curves. `csp_max` is the largest BT.601 x `P` coefficient at hue 0, in 1/256 units.
A matrix, three curves, and `P` cannot copy hue-selective changes. Examine the errors before you use an imported effect.
