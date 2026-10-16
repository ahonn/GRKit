# Starter effect attribution

Mono Orange and Everyday Neg are original effect definitions by GRKit
contributors. They are not measured film emulations.

Ektar 100, Pro 400H, and Vision3 250D derive from spektrafilm v0.3.4.
Andrea Volpato maintains [spektrafilm](https://github.com/andreavolpato/spektrafilm). The GR III parameter fits
are by idontknownihil (https://github.com/idontknownihil/gr3-film-mods), revision
04d5d5dd6e85d80089c6200bd0f0de5fff2d658f. SOURCES.json identifies the exact source
files and SHA256 hashes.

SPEKTRAFILM_LICENSE.txt contains the upstream notice.
CHANGELOG.txt preserves the upstream changes. The original fit inputs are
available in the source repository, not in this SD package.

The three imported effects target Ektar 100 on Portra Endura print, Pro 400H on
Crystal Archive print, and Vision3 250D on Kodak 2383 print. The upstream
changelog also describes sources that are not included in this starter set.

All five starter effect definitions and local effect-data adaptations use
CC BY-SA 4.0. This statement does not set a license for project code or artwork.
Local changes quantize each matrix to signed Q13 with ties-to-even rounding and
correct its diagonal to a row sum of 8192. All imported curve samples remain
unchanged.

The two original designs add their own matrices and tone curves.
Mono Orange requests monochrome processing. Each GRFX0001 file includes its
stable ID and an original menu icon.

Icon text uses the Inter typeface (SIL Open Font License 1.1, see
`crates/grkit-core/assets/Inter-LICENSE.txt`). Icons use no manufacturer logo.

These effects operate in the camera's native image-processing domain. They are
not display-sRGB JPEG transforms. No grain, spatial effect, post-curve matrix,
or hue-sector correction is included. Imported fits assume 18% gray at
base-gamma index 7.8. Film and paper names identify targets, not endorsement or
measured rendering equivalence.
