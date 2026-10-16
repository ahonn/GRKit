# Image Control effect sources

Each directory with `effect.json` defines one effect. The directory name is its stable effect ID.
Edit the name, signed Q13 matrix, RGB curves or curve knots, and icon to change an effect. Add a directory to add an effect.

The compiler writes self-contained GRFX0001 files with a 40 x 40 icon. CRC32 detects corruption. Neither the CRC nor the ID authenticates an author.
`golden.json` pins the compiled file and artwork hashes. Change it only for an intentional definition or artwork change.

## Starter effects

- Mono Orange and Everyday Neg are original definitions with editable curve knots.
- Ektar 100, Pro 400H, and Vision3 250D are imported fits with all 768 curve samples.

`sources/` stores the exact fit inputs, hashes, and upstream notices. `import_fit()` reproduces the imported matrices and curves.
Attribution and licenses are in [NOTICE.md](NOTICE.md). The installation guide in each package links to it.

## Processing domain

Effects run on the camera's look matrix before Standard base gamma. RGB curves index base-gamma output divided by 64.
They are not display-sRGB JPEG transforms. Imported fits assume 18% gray at base-gamma index 7.8.
Effects add no grain, spatial effects, post-curve matrices, or hue-sector correction.
