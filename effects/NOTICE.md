# Effect attribution

Each film effect is a fit of one film simulation LUT to the GR III effect model: a 3 x 3 look
matrix, three tone curves, and an optional post-curve matrix. GRKit contributors made the fits.
The repository and the SD packages contain the fitted parameters, not the LUTs.

## spektrafilm

Portra 400, Gold 200, Ektar 100, Pro 400H, X-Tra 400, Vision3 250D, Vision3 500T,
Portra 800 +2, Kodachrome 64, Velvia 100, and Portra 160 derive from LUTs of
[spektrafilm](https://github.com/andreavolpato/spektrafilm) 0.3.4 by Andrea Volpato,
licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
These effect definitions are also licensed under CC BY-SA 4.0.

`sources/SPEKTRAFILM_LICENSE.txt` contains the upstream notice. `sources/CHANGELOG.txt` describes
the changes: GRKit scans the two slide films directly instead of printing them, and fits each
LUT to the effect model.

## Spectral Film LUT

Tri-X 400, Tri-X 400 Red, FP-100C, Aerocolor IV, and EXR 200T derive from LUTs of
[Spectral Film LUT](https://github.com/JanLohse/spectral_film_lut) by Jan Lohse, licensed under
the MIT License. `sources/SPECTRAL_FILM_LUT_LICENSE.txt` contains its copyright and permission
notice. These effect definitions are also licensed under the MIT License. Tri-X 400 Red applies
the library's Kodak Wratten 25 data to Tri-X 400.

## Names and limits

Effect names and film, paper, and filter names identify the simulated materials. Some of these
names are trademarks of their owners. GRKit is not affiliated with or endorsed by Eastman Kodak Company,
Kodak Alaris, or Fujifilm. The simulations are models of published data, not measured
equivalents of the products.

The effects operate in the camera's native image-processing domain. They are not display-sRGB
JPEG transforms. They add no grain, halation, or other spatial effect. The fits assume 18% gray
at base-gamma index 7.8.

Icon text uses the Inter typeface (SIL Open Font License 1.1, see
`crates/grkit-core/assets/Inter-LICENSE.txt`). Icons use no manufacturer logo.
These statements do not set a license for project code or artwork.
